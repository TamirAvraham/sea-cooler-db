use std::{
    cell::RefCell,
    char::MAX,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    mem::size_of,
};

use crate::{
    error::{map_err, Error, InternalResult},
    node::{self, Node, MAX_KEY_SIZE},
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
    nodes_file: PagerFile,
    values_file: PagerFile,
    max_page_id: usize,
}

impl Pager {
    #[inline]
    fn file_to_pager_file(file: File) -> PagerFile {
        RefCell::new(file)
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

    pub fn new(nodes_file: File, values_file: File) -> Pager {
        Pager {
            nodes_file: Self::file_to_pager_file(nodes_file),
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

    pub fn new_page(&mut self) -> InternalResult<usize> {
        let mut file = self.nodes_file.borrow_mut();

        file.seek(std::io::SeekFrom::End(0))
            .map_err(map_err(Error::CantWriteValue))?;

        file.write_all(&[0; PAGE_SIZE])
            .map_err(map_err(Error::CantWriteValue))?;

        self.max_page_id += 1;

        Ok(self.max_page_id)
    }
    pub fn delete_node(&mut self,page_id: usize)->InternalResult<()>{
        let mut file = self.nodes_file.borrow_mut();

        file.seek(std::io::SeekFrom::Start((PAGE_SIZE * (page_id - 1)) as u64))
            .map_err(map_err(Error::CantGetNode(page_id)))?;

        file.write_all(&EMPTY_PAGE).map_err(map_err(Error::CantGetNode(page_id)))?;
        Ok(())
    }
    pub fn read_node(&self, page_id: usize) -> InternalResult<Node> {
        let mut page = [0; PAGE_SIZE];
        let mut file = self.nodes_file.borrow_mut();

        file.seek(std::io::SeekFrom::Start((PAGE_SIZE * (page_id - 1)) as u64))
            .map_err(map_err(Error::CantGetNode(page_id)))?;

        file.read_exact(&mut page)
            .map_err(map_err(Error::CantGetNode(page_id)))?;

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

    pub fn write_node(&mut self, node: &Node) -> InternalResult<()> {
        let mut page = self.transfer_node_to_bytes(node)?;
        let mut file = self.nodes_file.borrow_mut();

        file.seek(std::io::SeekFrom::Start(
            (PAGE_SIZE * (node.page_id - 1)) as u64,
        ))
        .map_err(map_err(Error::CantWriteNode(node.page_id)))?;

        file.write_all(&page)
            .map_err(map_err(Error::CantWriteNode(node.page_id)))?;
        Ok(())
    }
    pub fn get_node_max_key(&self, page_id: usize) -> InternalResult<String> {
        let mut page = [0; PAGE_SIZE];
        let mut file = self.nodes_file.borrow_mut();

        file.seek(std::io::SeekFrom::Start((PAGE_SIZE * (page_id - 1)) as u64))
            .map_err(map_err(Error::CantGetNode(page_id)))?;

        file.read_exact(&mut page)
            .map_err(map_err(Error::CantGetNode(page_id)))?;

        let key_count = usize::from_be_bytes(
            (&page[NODE_KEY_COUNT_OFFSET..NODE_KEY_COUNT_OFFSET + NODE_KEY_COUNT_SIZE])
                .try_into()
                .map_err(map_err(Error::CantGetNode(page_id)))?,
        );

        let key_start = (key_count - 1) * MAX_KEY_SIZE;

        Ok(
            String::from_utf8(page[key_start..MAX_KEY_SIZE + key_start].to_vec())
                .map_err(map_err(Error::CantGetNode(page_id)))?,
        )
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

    Pager::new(nodes_file, values_file)
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
        let node_page_id = pager.new_page().expect("cant create new page");
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
