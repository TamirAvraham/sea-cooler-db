use std::{
    cell::Cell,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::{Mutex, RwLock, RwLockWriteGuard},
};

use crate::{
    error::{map_err, Error, InternalResult},
    node::{Node, MAX_KEY_SIZE},
    pager::{
        HEADER_SIZE, NODE_KEY_COUNT_OFFSET, NODE_KEY_COUNT_SIZE, NODE_PARENT_OFFSET,
        NODE_PARENT_SIZE, NODE_TYPE_OFFSET, PAGE_SIZE, SIZE_OF_USIZE,
    },
};
const EMPTY_PAGE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
#[derive(Debug)]
pub struct FileCache {
    file: RwLock<File>,
    cache_size: usize,
    cache: RwLock<Vec<u8>>,
    start: Mutex<usize>,
    end: Mutex<usize>,
    current_file_page_count: Mutex<usize>,
    empty_pages: Vec<usize>,
}

impl FileCache {
    #[cfg(test)]
    fn get_page_for_tests(&self, page_id: usize) -> Vec<u8> {
        return if let Some(id) = self.relative_id(page_id) {
            let cache = self.cache.read().unwrap();
            cache[id..id + PAGE_SIZE].to_vec()
        } else {
            let mut cache = self.cache.write().unwrap();
            self.move_cache(page_id * PAGE_SIZE, &mut cache).unwrap();
            cache[0..PAGE_SIZE].to_vec()
        };
    }
    pub fn new(page_size: usize, mut file: File) -> Self {
        let current_file_page_count = (file.metadata().unwrap().len() as usize) / PAGE_SIZE;

        let mut empty_page_ids = vec![];

        if current_file_page_count < page_size {
            let new_page_count = page_size - current_file_page_count;

            for _ in 0..new_page_count {
                file.write_all(&EMPTY_PAGE).unwrap();
            }

            for page_id in (new_page_count - 1..=page_size - 1) {
                empty_page_ids.push(page_id) //mybe make this cleaner
            }
        }
        let current_file_page_count = (file.metadata().unwrap().len() as usize) / PAGE_SIZE;

        file.seek(SeekFrom::Start(0)).unwrap();
        for page_id in 0..current_file_page_count {
            let mut page = EMPTY_PAGE;
            file.read_exact(&mut page).unwrap();

            if page == EMPTY_PAGE {
                empty_page_ids.push(page_id)
            }
        }

        let mut cache = vec![0; PAGE_SIZE * page_size];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut cache).unwrap();

        Self {
            file: RwLock::new(file),
            cache_size: page_size,
            cache: RwLock::new(cache),
            start: Mutex::new(0),
            end: Mutex::new(page_size - 1),
            current_file_page_count: Mutex::new(current_file_page_count),
            empty_pages: empty_page_ids,
        }
    }
    ///# Description
    /// function takes a node and converts it to a page size slice of bytes that represents the node
    /// # Arguments
    ///
    /// * `node`: node to be translated to bytes
    ///
    /// returns: Result<[u8; 4096], Error>
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
    /// #  Description
    ///  function returns the id of a page in the cache if it is in it
    /// # Arguments
    ///
    /// * `page_id`: page id in file
    ///
    /// returns: Option<usize>

    #[inline]
    fn relative_id(&self, page_id: usize) -> Option<usize> {
        let (start, end) = (self.start.lock().unwrap(), self.end.lock().unwrap());
        if page_id >= *start && page_id <= *end {
            //start<page_id<end?
            //16,12,14,5,2
            Some(page_id - *start)
        } else {
            None
        }
    }
    /// # Description
    /// function takes a write guard of the cache and writes the cache to file
    /// # Arguments
    ///
    /// * `cache`: write guard of a cache
    ///
    /// returns: Result<(), Error> (if there was an error writing cache to disk)
    ///
    #[inline]
    fn write_cache_to_file(&self, cache: &mut RwLockWriteGuard<'_, Vec<u8>>) -> InternalResult<()> {
        let mut file = self.file.write().map_err(map_err(Error::MovingCacheError(
            *self.start.lock().unwrap(),
        )))?;

        file.seek(SeekFrom::Start(
            (PAGE_SIZE * *self.start.lock().unwrap()) as u64,
        ))
        .map_err(map_err(Error::FileError))?;

        file.write_all(&cache).map_err(map_err(Error::FileError))?;
        Ok(())
    }
    /// #  Description
    /// function moves the cache around from it's current location to the start param
    /// # Arguments
    ///
    /// * `start`: new start of the cache
    /// * `cache`: write guard of the cache
    ///
    /// returns: Result<(), Error>
    ///
    pub fn move_cache(
        &self,
        start: usize,
        cache: &mut RwLockWriteGuard<'_, Vec<u8>>,
    ) -> InternalResult<()> {
        self.write_cache_to_file(cache)?;

        let mut file = self
            .file
            .write()
            .map_err(map_err(Error::MovingCacheError(start)))?;

        let new_cache = if *self.current_file_page_count.lock().unwrap() < start + self.cache_size {
            let new_page_count =
                (start + self.cache_size) - *self.current_file_page_count.lock().unwrap();
            let mut new_cache = vec![];

            if new_page_count < self.cache_size {
                let amount_of_pages_i_already_have =
                    *self.current_file_page_count.lock().unwrap() % self.cache_size;

                file.seek(SeekFrom::End(0))
                    .map_err(map_err(Error::MovingCacheError(start)))?;

                new_cache.reserve(amount_of_pages_i_already_have);

                file.read_exact(&mut new_cache)
                    .map_err(map_err(Error::MovingCacheError(start)))?;
            }

            new_cache.extend(vec![0; new_page_count * PAGE_SIZE]);
            *self.current_file_page_count.lock().unwrap() = start + self.cache_size;
            new_cache
        } else {
            file.seek(SeekFrom::Start(start as u64))
                .map_err(map_err(Error::MovingCacheError(start)))?;
            let mut new_cache = vec![0; self.cache_size * PAGE_SIZE];
            file.read_exact(&mut new_cache)
                .map_err(map_err(Error::MovingCacheError(start)))?;
            new_cache
        };

        *(*cache) = new_cache;

        *self.start.lock().unwrap() = start;
        *self.end.lock().unwrap() = start + self.cache_size;

        Ok(())
    }

    /// #  Description
    ///  function writes the node to cache/file
    /// # Arguments
    ///
    /// * `node`: node to write
    ///
    /// returns: Result<(), Error> (if there was an error writing node to cache)
    ///
    pub fn write_node(&mut self, node: &Node) -> InternalResult<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(map_err(Error::CantWriteNode(node.page_id)))?;

        let relative_id = if let Some(page_id) = self.relative_id(node.page_id) {
            page_id * PAGE_SIZE
        } else {
            self.move_cache(node.page_id * PAGE_SIZE, &mut cache)?;
            0
        };

        //critical section
        {
            let mut file = self
                .file
                .write()
                .map_err(map_err(Error::CantWriteNode(node.page_id)))?;
            cache[relative_id..relative_id + PAGE_SIZE]
                .copy_from_slice(&self.transfer_node_to_bytes(node)?);
        }

        self.write_cache_to_file(&mut cache)?;

        Ok(())
    }
    /// #  Description
    ///     function deletes a page(cleans its data)
    /// # Arguments
    ///
    /// * `page_id`: page id to delete
    ///
    /// returns: Result<(), Error> (if there was an error deleting page)

    pub fn delete_page(&mut self, page_id: usize) -> InternalResult<()> {
        let mut cache = self
            .cache
            .write()
            .map_err(map_err(Error::CantDeletePage(page_id)))?;
        let relative_id = if let Some(page_id) = self.relative_id(page_id) {
            page_id * PAGE_SIZE
        } else {
            self.move_cache(page_id * PAGE_SIZE, &mut cache)?;
            0
        };

        cache[relative_id..relative_id + PAGE_SIZE].copy_from_slice(&EMPTY_PAGE);
        Ok(())
    }
    /// #   Description
    /// function deletes node's page
    /// # Arguments
    ///
    /// * `node`: node to delete it's page
    ///
    /// returns: Result<(), Error> (if there was an error deleting node's page)
    pub fn delete_node(&mut self, node: &Node) -> InternalResult<()> {
        self.delete_page(node.page_id)
            .map_err(map_err(Error::CantDeleteNode(node.page_id)))
    }
    ///# Description
    /// function reads a node from the cache
    /// # Arguments
    ///
    /// * `page_id`: page id of the node
    ///
    /// returns: Result<Node, Error> (if there was an error reading node from cache)
    pub fn read_node(&self, page_id: usize) -> InternalResult<Node> {
        let relative_id = if let Some(page_id) = self.relative_id(page_id) {
            page_id * PAGE_SIZE
        } else {
            let mut cache = self
                .cache
                .write()
                .map_err(map_err(Error::CantReadNode(page_id)))?;
            self.move_cache(page_id * PAGE_SIZE, &mut cache)?;
            0
        };

        let page = {
            let cache = self
                .cache
                .read()
                .map_err(map_err(Error::CantReadNode(page_id)))?;
            (&cache[relative_id..relative_id + PAGE_SIZE]).to_vec()
        };
        let is_leaf = match page[NODE_TYPE_OFFSET] {
            0x01 => Ok(true),
            0x00 => Ok(false),
            _ => Err(Error::CantReadNode(page_id)),
        }?;

        let parent = usize::from_be_bytes(
            (&page[NODE_PARENT_OFFSET..NODE_PARENT_OFFSET + NODE_PARENT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantReadNode(page_id)))?,
        );

        let key_count = usize::from_be_bytes(
            (&page[NODE_KEY_COUNT_OFFSET..NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantReadNode(page_id)))?,
        );

        let mut keys_vec = vec!["".to_string(); key_count];
        let mut key_offset = HEADER_SIZE;

        let mut values_vec = vec![0; key_count];
        let mut values_offset = key_offset + MAX_KEY_SIZE * key_count;

        for i in 0..key_count {
            let key = String::from_utf8(page[key_offset..MAX_KEY_SIZE + key_offset].to_vec())
                .map_err(map_err(Error::CantReadNode(page_id)))?
                .trim_end_matches('\0')
                .to_string();

            let value = usize::from_be_bytes(
                (&page[values_offset..values_offset + SIZE_OF_USIZE])
                    .try_into()
                    .map_err(map_err(Error::CantReadNode(page_id)))?,
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
                    .map_err(map_err(Error::CantReadNode(page_id)))?,
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
    pub fn new_node(&mut self) -> InternalResult<Node> {
        let page_id = if let Some(page_id) = self.empty_pages.pop() {
            page_id
        } else {
            *self.end.lock().unwrap() + 1
        };

        let new_node = Node {
            parent_page_id: 0,
            page_id,
            keys: vec![],
            values: vec![],
            is_leaf: true,
        };

        self.write_node(&new_node)?;

        Ok(new_node)
    }
    /// #  Description
    /// function reads the max key from a node (last key cause the tree is sorted)
    /// # Arguments
    ///
    /// * `page_id`: page id of the node
    ///
    /// returns: Result<String, Error> (if there was an error reading max key from node)

    pub fn get_node_max_key(&self, page_id: usize) -> InternalResult<String> {
        let relative_id = if let Some(page_id) = self.relative_id(page_id) {
            page_id
        } else {
            let mut cache = self
                .cache
                .write()
                .map_err(map_err(Error::CantReadNode(page_id)))?;
            self.move_cache(page_id * PAGE_SIZE, &mut cache)?;
            0
        };
        let start = PAGE_SIZE * relative_id;

        let cache = self
            .cache
            .read()
            .map_err(map_err(Error::CantReadNode(page_id)))?;

        let key_count = usize::from_be_bytes(
            (&cache[start + NODE_KEY_COUNT_OFFSET
                ..start + NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantReadNode(page_id)))?,
        );

        let key_start = HEADER_SIZE + key_count * MAX_KEY_SIZE;

        Ok(
            String::from_utf8(cache[start + key_start..start + MAX_KEY_SIZE + key_start].to_vec())
                .map_err(map_err(Error::CantReadNode(page_id)))?
                .trim_end_matches('\0')
                .to_string(),
        )
    }

    /// #   Description
    /// function updates the parent of a node
    /// # Arguments
    ///
    /// * `page_id`: node page id
    /// * `new_parent_id`: new parent page id
    ///
    /// returns: Result<(), Error> (if there was an error updating node parent)

    pub fn update_node_parent(
        &mut self,
        page_id: usize,
        new_parent_id: usize,
    ) -> Result<(), Error> {
        let relative_id = if let Some(page_id) = self.relative_id(page_id) {
            page_id
        } else {
            let mut cache = self
                .cache
                .write()
                .map_err(map_err(Error::CantReadNode(page_id)))?;
            self.move_cache(page_id * PAGE_SIZE, &mut cache)?;
            0
        };
        let start = PAGE_SIZE * relative_id;

        let mut cache = self
            .cache
            .write()
            .map_err(map_err(Error::CantReadNode(page_id)))?;

        cache[start + NODE_PARENT_OFFSET..start + NODE_PARENT_OFFSET + NODE_PARENT_SIZE]
            .clone_from_slice(&new_parent_id.to_be_bytes());

        self.write_cache_to_file(&mut cache)
    }
    pub fn new_page(&mut self) -> usize {
        if let Some(page_id) = self.empty_pages.pop() {
            page_id
        } else {
            *self.end.lock().unwrap() + 1
        }
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
        let mut data_lock = cache.cache.write().unwrap();
        assert_eq!(*cache.current_file_page_count.lock().unwrap(), 5);

        cache.move_cache(10, &mut data_lock).unwrap();
        assert_eq!(*cache.current_file_page_count.lock().unwrap(), 15);

        cache.move_cache(0, &mut data_lock).unwrap();
        assert_eq!(*cache.current_file_page_count.lock().unwrap(), 15);

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

        let cache = get_file_cache(page_size);

        let mut data = cache.cache.write().unwrap();

        let read_start = *cache.end.lock().unwrap() * PAGE_SIZE;

        data[read_start..read_start + 8].copy_from_slice(&value.to_be_bytes());
        cache.move_cache(page_size * 2, &mut data).unwrap();

        let cache = get_file_cache_no_create(page_size);
        let read_value = usize::from_be_bytes(
            (&cache.cache.read().unwrap()[read_start..read_start + 8])
                .try_into()
                .unwrap(),
        );

        assert_eq!(value, read_value);
    }
    #[test]
    fn test_node_new_node() {
        let page_size = 5;
        let mut cache = get_file_cache(page_size);
        let new_node = cache.new_node();
        assert_eq!(
            new_node,
            Ok(Node {
                parent_page_id: 0,
                page_id: page_size - 1,
                keys: vec![],
                values: vec![],
                is_leaf: true
            })
        );
        let new_node = new_node.unwrap();
        let node_from_file = cache.read_node(new_node.page_id).unwrap();

        assert_eq!(new_node, node_from_file)
    }
}
