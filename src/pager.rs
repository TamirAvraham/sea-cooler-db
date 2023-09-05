use std::{
    cell::RefCell,
    char::MAX,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    mem::size_of,
};

use crate::{
    error::{map_err, Error, InternalResult},
    node::{self, Node, MAX_KEY_SIZE}, page_cache::FileCache,
};

pub const PAGE_SIZE: usize = 1024 * 4;
pub const SIZE_OF_USIZE: usize = size_of::<usize>();
const EMPTY_PAGE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];
/*
value structure
[value_len - 8b][parent_id - 8b][key_location -8b][value - value_len b]
 */

pub const NODE_TYPE_OFFSET: usize = 0;
pub const NODE_TYPE_SIZE: usize = 1;
pub const NODE_PARENT_SIZE: usize = SIZE_OF_USIZE;
pub const NODE_PARENT_OFFSET: usize = NODE_TYPE_SIZE;
pub const NODE_KEY_COUNT_SIZE: usize = SIZE_OF_USIZE;
pub const NODE_KEY_COUNT_OFFSET: usize = NODE_PARENT_OFFSET + NODE_PARENT_SIZE;
pub const HEADER_SIZE: usize = NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE;

type PagerFile = RefCell<File>;
#[derive(Debug)]
pub struct Pager {
    nodes_cache: FileCache,
    values_file: PagerFile,
    max_page_id: usize,
}

impl Pager {
    #[inline]
    fn file_to_pager_file(file: File) -> PagerFile {
        RefCell::new(file)
    }

    
    pub fn new(nodes_file: File, values_file: File,cache_size: usize) -> Pager {
        Pager {
            nodes_cache: FileCache::new(cache_size,nodes_file),
            values_file: Self::file_to_pager_file(values_file),
            max_page_id: 0,
        }
    }
    pub fn read_value(&self, location: usize) -> InternalResult<Vec<u8>> {
        let mut file = self.values_file.borrow_mut();

        file.seek(std::io::SeekFrom::Start(location as u64))
            .map_err(map_err(Error::CantWriteValue))?;

        let mut value_len_as_bytes = [0; SIZE_OF_USIZE];

        file.read_exact(&mut value_len_as_bytes)
            .map_err(map_err(Error::CantWriteValue))?;

        let value_len = usize::from_be_bytes(value_len_as_bytes);
        let mut value = vec![0; value_len];

        file.read_exact(&mut value)
            .map_err(map_err(Error::CantWriteValue))?;

        Ok(value)
    }
    pub fn new_value(&mut self, value: &[u8]) -> InternalResult<usize> {
        let mut file = self.values_file.borrow_mut();

        file.seek(std::io::SeekFrom::End(0))
            .map_err(map_err(Error::CantWriteValue))?;

        let ret = file.metadata().unwrap().len();
        let value_len = value.len().to_be_bytes();

        file.write_all(&value_len)
            .map_err(map_err(Error::CantWriteValue))?;

        file.write_all(value)
            .map_err(map_err(Error::CantWriteValue))?;

        Ok(ret as usize)
    }

    pub fn delete_value(&mut self, offset: usize) -> Result<(), Error> {
        let mut file = self.values_file.borrow_mut();
        let mut value_len_as_bytes = [0; SIZE_OF_USIZE];

        file.seek(std::io::SeekFrom::Start(offset as u64)).map_err(|_| Error::FileError)?;
        file.read_exact(&mut value_len_as_bytes).map_err(|_| Error::FileError)?;

        let value_len: usize = usize::from_be_bytes(value_len_as_bytes);
        let new_empty_value = vec![0; value_len];
        file.seek(std::io::SeekFrom::Start((offset + SIZE_OF_USIZE) as u64)).map_err(|_| Error::FileError)?;

        file.write_all(&new_empty_value).map_err(|_| Error::FileError)?;
        Ok(())
    }

    pub fn rebalance(pager: &mut Pager, node: &mut Node, t: usize) -> Result<(), Error> {
        let mut parent = node.get_parent(pager).unwrap();
        
        if parent.is_leaf {
            return Err(Error::ParentError);
        }

        let node_index_in_parent;
        if node.keys.is_empty()&&node.values.is_empty(){
            node_index_in_parent = 0;

        } else {
            let key = node.keys.last().unwrap().clone();

            let mut i = 0;
            while parent.keys.len() > i && parent.keys[i] < key {
                i += 1
            }
            node_index_in_parent=i;
        }

        

        let sibling_page_id_index;
        let smaller_index;

        match parent.keys.len().saturating_sub(1) == node_index_in_parent {
            true => {
                sibling_page_id_index = node_index_in_parent - 1;
                smaller_index=sibling_page_id_index
            }
            false => {
                sibling_page_id_index = node_index_in_parent + 1;
                smaller_index = node_index_in_parent;
            }
        }

        return Ok(());
    }

    pub fn delete_node(&mut self,page_id: usize)->InternalResult<()>{
        self.nodes_cache.delete_page(page_id)
    }
    pub fn read_node(&self, page_id: usize) -> InternalResult<Node> {
        self.nodes_cache.read_node(page_id)
    }

    pub fn write_node(&mut self, node: &Node) -> InternalResult<()> {
        self.nodes_cache.write_node(node)
    }
    pub fn new_node(&mut self)->InternalResult<Node>{
        self.nodes_cache.new_node()
    }
    pub fn new_page(&mut self)->usize{
        self.nodes_cache.new_page()
    }
    pub fn get_node_max_key(&self,page_id: usize) -> Result<String, Error> {
        self.nodes_cache.get_node_max_key(page_id)
    }
}
#[cfg(test)]
pub fn delete_file(file_path: &'static str) -> Option<()> {
    if fs::metadata(&file_path).is_ok() {
        fs::remove_file(&file_path).ok()?;
    }
    Some(())
}

#[cfg(test)]
pub fn create_pager() -> Pager {
    let (nodes_file_name, vlaues_file_name) = ("nodes_test_file.bin", "values_test_file.bin");

    delete_file(nodes_file_name).unwrap();
    delete_file(vlaues_file_name).unwrap();

    let mut file_options = OpenOptions::new();
    file_options.read(true).write(true).create(true);

    let nodes_file = file_options.open(&nodes_file_name).unwrap();
    let values_file = file_options.open(&vlaues_file_name).unwrap();
    let cache_size=5;
    Pager::new(nodes_file, values_file,cache_size)
}
#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_can_create_pager() {
        create_pager();
    }

    #[test]
    fn read_and_writer_value() {
        let mut pager = create_pager();

        let value = "test".to_string();

        let value_loaction = pager.new_value(value.as_bytes()).expect("cant wrtie value");
        let read_value =
            String::from_utf8(pager.read_value(value_loaction).expect("cant read value"))
                .expect("UTF-8 error");

        assert_eq!(value, read_value)
    }

    #[test]
    fn test_node_read_write() {
        let mut pager = create_pager();
        let node_page_id = pager.new_page();
        let node = Node {
            parent_page_id: 10,
            page_id: node_page_id,
            keys: vec!["1".to_string(), "2".to_string(), "4".to_string()],
            values: vec![3, 9, 1],
            is_leaf: true,
        };

        pager.write_node(&node).expect("cant write node");

        let read_node = pager.read_node(node_page_id).expect("cant read node");

        assert_eq!(node, read_node)
    }
}
