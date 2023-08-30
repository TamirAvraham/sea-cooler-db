use std::{
    cell::Cell,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::{RwLock, RwLockWriteGuard},
};

use crate::{
    error::{map_err, Error, InternalResult},
    pager::{PAGE_SIZE, NODE_TYPE_OFFSET, NODE_PARENT_OFFSET, NODE_PARENT_SIZE, NODE_KEY_COUNT_OFFSET, NODE_KEY_COUNT_SIZE, HEADER_SIZE, SIZE_OF_USIZE}, node::{Node, MAX_KEY_SIZE},
};
const EMPTY_PAGE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

struct FileCache {
    file: RwLock<File>,
    page_size: usize,
    cache: RwLock<Vec<u8>>,
    start: Cell<usize>,
    end: Cell<usize>,
    current_file_page_count: Cell<usize>,
    empty_pages:Vec<usize>
}

impl FileCache {
    #[cfg(test)]
    fn get_page_for_tests(&self,page_id: usize)->Vec<u8> {
        return if let Some(id) = self.relative_id(page_id) {
            let cache=self.cache.read().unwrap();
            cache[id..id+PAGE_SIZE].to_vec()
        }else{
            let mut cache=self.cache.write().unwrap();
            self.move_cache(page_id, &mut cache).unwrap();
            cache[0..PAGE_SIZE].to_vec()
        }
    }
    pub fn new(page_size: usize, mut file: File) -> Self {
        let current_file_page_count = (file.metadata().unwrap().len() as usize) / PAGE_SIZE;

        let mut empty_page_ids=vec![];
        
        if current_file_page_count < page_size {
            
            let new_page_count = page_size - current_file_page_count;

            for _ in 0..new_page_count {
                file.write_all(&EMPTY_PAGE).unwrap();
            }

            for page_id in (new_page_count-1..=page_size-1){
                empty_page_ids.push(page_id)//mybe make this cleaner
            }
        }
        let current_file_page_count = (file.metadata().unwrap().len() as usize) / PAGE_SIZE;
        
        file.seek(SeekFrom::Start(0)).unwrap();
        for page_id in 0..current_file_page_count{
            let mut page=EMPTY_PAGE;
            file.read_exact(&mut page).unwrap();

            if page==EMPTY_PAGE {
                empty_page_ids.push(page_id)
            }
        }

        let mut cache = vec![0; PAGE_SIZE * page_size];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut cache).unwrap();

        Self {
            file: RwLock::new(file),
            page_size,
            cache: RwLock::new(cache),
            start: Cell::new(0),
            end: Cell::new(page_size-1),
            current_file_page_count: Cell::new(current_file_page_count),
            empty_pages: empty_page_ids,
        }
    }
    #[inline]
    fn transfer_node_to_bytes(&self, node: &Node) -> InternalResult<[u8; PAGE_SIZE]> {
        let mut page = [0; PAGE_SIZE];
        page[NODE_TYPE_OFFSET] = match node.is_leaf {
            true => 0x01,
            false => 0x00,
        };

        page[NODE_PARENT_OFFSET..NODE_PARENT_OFFSET + NODE_PARENT_SIZE]
            .clone_from_slice(&node.parent_page_id.to_be_bytes());
        page[NODE_KEY_COUNT_OFFSET..NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE]
            .clone_from_slice(&node.keys.len().to_be_bytes());
        let mut offset = HEADER_SIZE;

        for key in &node.keys {
            page[offset..offset + key.len()].clone_from_slice(key.as_bytes());
            offset += MAX_KEY_SIZE;
        }

        for value in &node.values {
            page[offset..offset + SIZE_OF_USIZE].clone_from_slice(&value.to_be_bytes());
            offset += SIZE_OF_USIZE;
        }
        Ok(page)
    }
    #[inline]
    fn relative_id(&self,page_id: usize)->Option<usize> {
        let (start,end)=(self.start.get(),self.end.get());
        if page_id>=start&&page_id<=end { //start<page_id<end?
            //16,12,14,5,2
            Some(page_id-start)
        }
        else {
            None
        }
    }
    #[inline]
    fn write_cache_to_file(
        &self,
        cache: &mut RwLockWriteGuard<'_, Vec<u8>>,
    ) -> InternalResult<()> {
        let mut file = self
            .file
            .write()
            .map_err(map_err(Error::MovingCacheError(self.start.get())))?;

        file.seek(SeekFrom::Start((PAGE_SIZE * self.start.get()) as u64))
            .map_err(map_err(Error::FileError))?;

        file.write_all(&cache).map_err(map_err(Error::FileError))?;
        Ok(())
    }
    pub fn move_cache(&self, start: usize,cache: &mut RwLockWriteGuard<'_, Vec<u8>>) -> InternalResult<()> {
        self.write_cache_to_file(cache)?;

        let mut file = self
            .file
            .write()
            .map_err(map_err(Error::MovingCacheError(start)))?;

        let new_cache = if self.current_file_page_count.get() < start + self.page_size {
            let new_page_count = (start + self.page_size) - self.current_file_page_count.get();
            let mut new_cache = vec![];

            if new_page_count < self.page_size {
                let amount_of_pages_i_already_have =
                    self.current_file_page_count.get() % self.page_size;

                file.seek(SeekFrom::End(0))
                    .map_err(map_err(Error::MovingCacheError(start)))?;

                new_cache.reserve(amount_of_pages_i_already_have);

                file.read_exact(&mut new_cache)
                    .map_err(map_err(Error::MovingCacheError(start)))?;
            }

            new_cache.extend(vec![0; new_page_count * PAGE_SIZE]);
            self.current_file_page_count.set(start + self.page_size);

            new_cache
        } else {
            file.seek(SeekFrom::Start(start as u64))
                .map_err(map_err(Error::MovingCacheError(start)))?;
            let mut new_cache = vec![0; self.page_size * PAGE_SIZE];
            file.read_exact(&mut new_cache)
                .map_err(map_err(Error::MovingCacheError(start)))?;
            new_cache
        };

        *(*cache) = new_cache;

        self.start.set(start);
        self.end.set(start + self.page_size);

        Ok(())
    }

    pub fn write_node(&mut self,node:&Node)->InternalResult<()>{
        let mut cache=self.cache.write()
            .map_err(map_err(Error::CantWriteNode(node.page_id)))?;

        let relative_id=if let Some(page_id) = self.relative_id(node.page_id) {
            page_id
        } else{
            self.move_cache(node.page_id, &mut cache)?;
            0
        };

        //critical section
        {
            let mut file=self.file.write()
                .map_err(map_err(Error::CantWriteNode(node.page_id)))?;
            cache[relative_id..relative_id+PAGE_SIZE].copy_from_slice(&self.transfer_node_to_bytes(node)?);
        }
        
        self.write_cache_to_file(&mut cache)?;

        Ok(())
    }
    pub fn delete_page(&mut self,page_id: usize)->InternalResult<()>{
        let mut cache=self.cache.write().map_err(map_err(Error::CantDeletePage(page_id)))?;
        let relative_id=if let Some(page_id) = self.relative_id(page_id) {
            page_id
        }else{
            self.move_cache(page_id, &mut cache)?;
            0
        };

        cache[relative_id..relative_id+PAGE_SIZE].copy_from_slice(&EMPTY_PAGE);
        Ok(())
    }
    pub fn delete_node(&mut self,node: &Node)->InternalResult<()>{
        self.delete_page(node.page_id).map_err(map_err(Error::CantDeleteNode(node.page_id)))
    }
    pub fn read_node(&self,page_id: usize)->InternalResult<Node>{
        let relative_id=if let Some(page_id) = self.relative_id(page_id) {
            page_id
        } else{
            let mut cache=self.cache.write()
            .map_err(map_err(Error::CantGetNode(page_id)))?;
            self.move_cache(page_id, &mut cache)?;
            0
        };

        let page={
            let cache=self.cache.read()
            .map_err(map_err(Error::CantGetNode(page_id)))?;
            let delete_this_after_debugged = &cache[relative_id..relative_id+PAGE_SIZE];
            delete_this_after_debugged.to_vec()
        };
        let is_leaf = match page[NODE_TYPE_OFFSET] {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(Error::CantGetNode(page_id)),
        }?;

        let parent = usize::from_be_bytes(
            (&page[NODE_PARENT_OFFSET..NODE_PARENT_OFFSET + NODE_PARENT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantGetNode(page_id)))?,
        );

        let key_count = usize::from_be_bytes(
            (&page[NODE_KEY_COUNT_OFFSET..NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantGetNode(page_id)))?,
        );

        let mut keys_vec = vec!["".to_string(); key_count];
        let mut key_offset = HEADER_SIZE;

        let mut values_vec = vec![0; key_count];
        let mut values_offset = key_offset + MAX_KEY_SIZE * key_count;

        for i in 0..key_count {
            let key = String::from_utf8(page[key_offset..MAX_KEY_SIZE + key_offset].to_vec())
                .map_err(map_err(Error::CantGetNode(page_id)))?
                .trim_end_matches('\0')
                .to_string();

            let value = usize::from_be_bytes(
                (&page[values_offset..values_offset + SIZE_OF_USIZE])
                    .try_into()
                    .map_err(map_err(Error::CantGetNode(page_id)))?,
            );

            keys_vec[i] = key;
            values_vec[i] = value;

            key_offset += MAX_KEY_SIZE;
            values_offset += SIZE_OF_USIZE;
        }

        if !is_leaf {
            let value = usize::from_be_bytes(
                (&page[values_offset..values_offset + SIZE_OF_USIZE])
                    .try_into()
                    .map_err(map_err(Error::CantGetNode(page_id)))?,
            );
            values_vec.push(value)
        }

        Ok(Node {
            parent_page_id: parent,
            page_id,
            keys: keys_vec,
            values: values_vec,
            is_leaf,
        })
    }
    pub fn new_node(&mut self)->InternalResult<Node>{
        let page_id=if let Some(page_id)=self.empty_pages.pop() {
            page_id
        }else{
            self.end.get()+1
            
        };

        let new_node=Node{ parent_page_id: 0, page_id, keys: vec![], values: vec![], is_leaf: true };

        self.write_node(&new_node)?;

        Ok(new_node)

    }
}

// i dont think this is a good idea but if we want this this is the correct impl for running write on drop
// impl Drop for FileCache {
//     fn drop(&mut self) {
//         let mut file=self.file.write().unwrap();
//         let mut cache=self.cache.write().unwrap();
//         self.write_cache_to_file(&mut file, &mut cache).unwrap();
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    fn get_file_cache_no_create(page_size: usize) -> FileCache {
        let path = "test_nodes_file.bin";

        let mut options = OpenOptions::new();
        options.read(true).write(true);

        let file = options.open(path).unwrap();
        FileCache::new(page_size, file)
    }
    fn get_file_cache(page_size: usize) -> FileCache {
        let path = "test_nodes_file.bin";

        if fs::metadata(path).is_ok() {
            fs::remove_file(path).unwrap()
        }

        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);

        let file = options.open(path).unwrap();
        FileCache::new(page_size, file)
    }
    #[test]
    fn test_move_cache() {
        let cache = get_file_cache(5);
        let mut data_lock=cache.cache.write().unwrap();
        assert_eq!(cache.current_file_page_count.get(), 5);

        cache.move_cache(10,&mut data_lock).unwrap();
        assert_eq!(cache.current_file_page_count.get(), 15);

        cache.move_cache(0,&mut data_lock).unwrap();
        assert_eq!(cache.current_file_page_count.get(), 15);


        //add more test cases like from 5 to 45 and make sure all the pages existed
    }
    #[test]
    fn test_for_cache_write() {
        let page_size = 5;
        let value = 8usize;

        let cache = get_file_cache(page_size);
        let mut data = cache.cache.write().unwrap();

        
        data[0..8].copy_from_slice(&value.to_be_bytes());
        cache.write_cache_to_file(&mut data).unwrap();

        let cache = get_file_cache_no_create(page_size);
        let read_value =
            usize::from_be_bytes((&cache.cache.read().unwrap()[0..8]).try_into().unwrap());
        
        assert_eq!(value, read_value);

        
        
    }
    #[test]
    fn write_on_move() {
        let page_size = 5;
        let value = 8usize;

        let cache=get_file_cache(page_size);
        
        let mut data = cache.cache.write().unwrap();
        
        let read_start=cache.end.get()*PAGE_SIZE;

        data[read_start..read_start+8].copy_from_slice(&value.to_be_bytes());
        cache.move_cache(page_size*2,&mut data).unwrap();
        

        
        let cache = get_file_cache_no_create(page_size);
        let read_value =
            usize::from_be_bytes((&cache.cache.read().unwrap()[read_start..read_start+8]).try_into().unwrap());
        

        assert_eq!(value, read_value);
    }
    #[test]
    fn test_node_new_node(){
        let page_size=5;
        let mut cache=get_file_cache(page_size);
        let new_node=cache.new_node();
        assert_eq!(new_node,Ok(Node{ 
            parent_page_id: 0, 
            page_id: page_size-1, 
            keys: vec![], 
            values: vec![], 
            is_leaf: true
        }));
        let new_node=new_node.unwrap();
        let node_from_file=cache.read_node(new_node.page_id).unwrap();

        assert_eq!(new_node,node_from_file)
    }

    
}
