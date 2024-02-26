use crate::skip_list::NodeType::Deleted;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::string::FromUtf8Error;
use std::sync::{Mutex, RwLock};
use std::usize;
const NULL_NODE_ID: usize = usize::MAX;
pub const SKIP_LIST_CONFIG_FILE_ENDING: &str = ".skiplist.config";
pub const SKIP_LIST_MAIN_FILE_ENDING: &str = ".skiplist.dat";
const SIZE_OF_USIZE: usize = size_of::<usize>();
const SIZE_OF_TYPE: usize = size_of::<u8>();
const SIZE_OF_KEY_SIZE: usize = SIZE_OF_USIZE;
const SIZE_OF_VALUE_SIZE: usize = SIZE_OF_USIZE;
const SIZE_OF_TOP_NODE: usize = SIZE_OF_USIZE;
const SIZE_OF_DOWN_NODE: usize = SIZE_OF_USIZE;
const SIZE_OF_PREV_NODE: usize = SIZE_OF_USIZE;
const SIZE_OF_NEXT_NODE: usize = SIZE_OF_USIZE;

const TYPE_OFFSET: usize = 0;
const NEXT_OFFSET: usize = TYPE_OFFSET + SIZE_OF_TYPE;
const PREV_OFFSET: usize = NEXT_OFFSET + SIZE_OF_USIZE;
const TOP_OFFSET: usize = PREV_OFFSET + SIZE_OF_USIZE;
const DOWN_OFFSET: usize = TOP_OFFSET + SIZE_OF_USIZE;
const KEY_SIZE_OFFSET: usize = DOWN_OFFSET + SIZE_OF_USIZE;
const VALUE_SIZE_OFFSET: usize = KEY_SIZE_OFFSET + SIZE_OF_USIZE;
const KEY_OFFSET: usize = VALUE_SIZE_OFFSET + SIZE_OF_USIZE;

const NODE_HEADER_SIZE: usize = KEY_OFFSET;
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
enum NodeType {
    Linker = 0,
    Data = 1,
    Deleted = 2,
}
#[derive(Debug)]
pub enum SkipListError {
    FileError(std::io::Error),
    Utf8Error(FromUtf8Error),
}

impl From<std::io::Error> for SkipListError {
    fn from(value: Error) -> Self {
        SkipListError::FileError(value)
    }
}

impl From<FromUtf8Error> for SkipListError {
    fn from(value: FromUtf8Error) -> Self {
        SkipListError::Utf8Error(value)
    }
}
struct Node {
    id: usize,
    node_type: NodeType,
    key: String,

    top_node: Option<usize>,
    down_node: Option<usize>,

    prev_node: Option<usize>, // left
    next_node: Option<usize>, // right

    value: Vec<usize>,
}
type Res<T> = Result<T, SkipListError>;
pub struct SkipList {
    rows: Vec<usize>,
    file_handler: SkipListFileHandler,
}

struct SkipListFileHandler {
    config_file: Mutex<File>,
    main_file: Mutex<File>,
}

impl NodeType {
    fn as_u8(&self) -> u8 {
        match &self {
            NodeType::Linker => 0,
            NodeType::Data => 1,
            Deleted => 2,
        }
    }
    fn from_u8(u: u8) -> NodeType {
        match u {
            0 => NodeType::Linker,
            1 => NodeType::Data,
            _ => Deleted,
        }
    }
}
/*
[type-1b][next - 8b][prev -8b][top -8b][down-8b][key_len-8b][value_len-8b][key- key_len b][value- value_len b]
[row_count - 8b][rows id -8b * row_count]
*/
impl SkipListFileHandler {
    fn update_config_file(&self, rows: &Vec<usize>) -> Res<()> {
        let mut config_file = self.config_file.lock().unwrap();
        config_file.seek(SeekFrom::Start(0))?;
        let rows_len = rows.len();
        let rows_len_bytes = rows_len.to_be_bytes();
        config_file.write_all(&rows_len_bytes)?;
        for row in rows {
            let row_bytes = row.to_be_bytes();
            config_file.write_all(&row_bytes)?;
        }
        Ok(())
    }
    fn read_config_file(&self) -> Res<Vec<usize>> {
        let mut config_file = self.config_file.lock().unwrap();
        config_file.seek(SeekFrom::Start(0))?;
        let mut rows_len_bytes = [0u8; SIZE_OF_USIZE];
        config_file.read_exact(&mut rows_len_bytes)?;
        let rows_len = usize::from_be_bytes(rows_len_bytes);

        let mut rows_bytes = vec![0u8; rows_len * SIZE_OF_USIZE];
        config_file.read_exact(&mut rows_bytes)?;

        Ok(rows_bytes
            .chunks(SIZE_OF_USIZE)
            .map(|x| usize::from_be_bytes(x.try_into().unwrap()))
            .collect())
    }
    fn new(name: &String) -> Res<SkipListFileHandler> {
        let mut open_options = OpenOptions::new();
        open_options.read(true).write(true).create(true);

        Ok(SkipListFileHandler {
            config_file: Mutex::new(
                open_options.open(&format!("{}{}", name, SKIP_LIST_CONFIG_FILE_ENDING))?,
            ),
            main_file: Mutex::new(
                open_options.open(&format!("{}{}", name, SKIP_LIST_MAIN_FILE_ENDING))?,
            ),
        })
    }
    #[inline]
    fn write_pointer_to_bytes_in_offset(
        &self,
        offset: usize,
        len: usize,
        bytes: &mut [u8],
        data: &Option<usize>,
    ) {
        self.write_usize_to_bytes_in_offset(offset, len, bytes, data.unwrap_or(NULL_NODE_ID));
    }
    fn write_usize_to_bytes_in_offset(
        &self,
        offset: usize,
        len: usize,
        bytes: &mut [u8],
        data: usize,
    ) {
        bytes[offset..(offset + len)].copy_from_slice(&(data.to_be_bytes()));
    }
    fn convert_node_to_header(&self, node: &Node) -> [u8; NODE_HEADER_SIZE] {
        let mut ret = [0; NODE_HEADER_SIZE];

        ret[TYPE_OFFSET] = node.node_type.as_u8();

        //write pointers
        self.write_pointer_to_bytes_in_offset(
            NEXT_OFFSET,
            SIZE_OF_NEXT_NODE,
            &mut ret,
            &node.next_node,
        );
        self.write_pointer_to_bytes_in_offset(
            PREV_OFFSET,
            SIZE_OF_PREV_NODE,
            &mut ret,
            &node.prev_node,
        );
        self.write_pointer_to_bytes_in_offset(
            TOP_OFFSET,
            SIZE_OF_TOP_NODE,
            &mut ret,
            &node.top_node,
        );
        self.write_pointer_to_bytes_in_offset(
            DOWN_OFFSET,
            SIZE_OF_DOWN_NODE,
            &mut ret,
            &node.down_node,
        );

        //write key and value len
        self.write_usize_to_bytes_in_offset(
            KEY_SIZE_OFFSET,
            SIZE_OF_KEY_SIZE,
            &mut ret,
            node.key.len(),
        );
        self.write_usize_to_bytes_in_offset(
            VALUE_SIZE_OFFSET,
            SIZE_OF_VALUE_SIZE,
            &mut ret,
            node.value.len() * SIZE_OF_USIZE,
        );
        ret
    }
    fn update_node_header(&self, node: &Node) -> Res<()> {
        let header = self.convert_node_to_header(node);

        {
            let mut file = self.main_file.lock().unwrap();
            file.seek(SeekFrom::Start(node.id as u64))?;
            file.write_all(&header)?;
        }

        Ok(())
    }
    fn new_node(&self, key: String, value: Option<Vec<usize>>) -> Res<Node> {
        let node_type = match value {
            Some(_) => NodeType::Data,
            None => NodeType::Linker,
        };
        let mut node = Node {
            id: NULL_NODE_ID,
            node_type,
            key: key.clone(),
            top_node: None,
            down_node: None,
            prev_node: None,
            next_node: None,
            value: value.unwrap_or(vec![]),
        };
        let node_header = self.convert_node_to_header(&node);
        let value = node
            .value
            .iter()
            .flat_map(|&x| x.to_be_bytes().to_vec())
            .collect::<Vec<u8>>();
        let id = {
            let mut file = self.main_file.lock().unwrap();
            let len = file.metadata().unwrap().len();
            file.seek(SeekFrom::End(0))?;
            file.write_all(&node_header)?;
            file.write_all(node.key.as_bytes())?;
            file.write_all(value.as_slice())?;

            len as usize
        };
        node.id = id;
        Ok(node)
    }
    fn read_pointer_from_bytes_in_offset(
        &self,
        offset: usize,
        len: usize,
        bytes: &[u8],
    ) -> Option<usize> {
        let data = usize::from_be_bytes(bytes[offset..(offset + len)].try_into().unwrap());
        if data == NULL_NODE_ID {
            None
        } else {
            Some(data)
        }
    }
    fn read_node(&self, node_id: usize) -> Res<Node> {
        let mut node_header = [0; NODE_HEADER_SIZE];

        {
            let mut file = self.main_file.lock().unwrap();

            file.seek(SeekFrom::Start(node_id as u64))?;
            file.read_exact(&mut node_header)?;
        }

        let node_type = NodeType::from_u8(node_header[TYPE_OFFSET]);

        let next =
            self.read_pointer_from_bytes_in_offset(NEXT_OFFSET, SIZE_OF_NEXT_NODE, &node_header);

        if node_type == NodeType::Deleted {
            return if let Some(next) = next {
                self.read_node(next) // if the node is deleted and there is a new implementation go to new node
            } else {
                Ok(Node {
                    id: NULL_NODE_ID,
                    node_type,
                    key: "".to_string(),
                    top_node: None,
                    down_node: None,
                    prev_node: None,
                    next_node: None,
                    value: vec![],
                })
            };
        }
        let mut file = self.main_file.lock().unwrap();
        file.seek(SeekFrom::Start((node_id + KEY_OFFSET) as u64))?;

        let key_len = usize::from_be_bytes(
            node_header[KEY_SIZE_OFFSET..(KEY_SIZE_OFFSET + SIZE_OF_KEY_SIZE)]
                .try_into()
                .unwrap(),
        );
        let mut key_bytes = vec![0; key_len];
        file.read_exact(&mut key_bytes)?;
        let key = String::from_utf8(key_bytes)?;
        let prev =
            self.read_pointer_from_bytes_in_offset(PREV_OFFSET, SIZE_OF_PREV_NODE, &node_header);
        let top =
            self.read_pointer_from_bytes_in_offset(TOP_OFFSET, SIZE_OF_TOP_NODE, &node_header);
        let down =
            self.read_pointer_from_bytes_in_offset(DOWN_OFFSET, SIZE_OF_DOWN_NODE, &node_header);

        Ok(if let NodeType::Data = node_type {
            let value_len = usize::from_be_bytes(
                node_header[VALUE_SIZE_OFFSET..(VALUE_SIZE_OFFSET + SIZE_OF_VALUE_SIZE)]
                    .try_into()
                    .unwrap(),
            );
            let mut value_bytes = vec![0; value_len];
            file.read_exact(&mut value_bytes)?;
            let value = value_bytes
                .chunks(SIZE_OF_USIZE)
                .map(|x| usize::from_be_bytes(x.try_into().unwrap()))
                .collect();
            Node {
                id: node_id,
                node_type,
                key,
                top_node: top,
                down_node: down,
                prev_node: prev,
                next_node: next,
                value,
            }
        } else {
            Node {
                id: node_id,
                node_type,
                key,
                top_node: top,
                down_node: down,
                prev_node: prev,
                next_node: next,
                value: vec![],
            }
        })
    }
    fn delete_node(&self, node: &mut Node, new_node_location: Option<usize>) -> Res<()> {
        node.node_type = NodeType::Deleted;
        node.next_node = new_node_location;
        self.update_node_header(node)
    }
    fn update_node_value(&self, node: &mut Node) -> Res<()> {
        let mut new_node = self.new_node(node.key.clone(), Some(node.value.clone()))?;

        new_node.next_node = node.next_node;
        new_node.prev_node = node.prev_node;
        new_node.top_node = node.top_node;
        new_node.down_node = node.down_node;
        self.update_node_header(&new_node)?;

        let ret = self.delete_node(node, Some(new_node.id));
        ret
    }
}

#[cfg(test)]
mod pager_tests {
    use super::*;
    use std::fs;

    const TEST_FILE: &str = "test_skip_list_pager.bin";

    fn setup() {
        let _ = fs::remove_file(TEST_FILE); // Clean up before each test
    }

    fn teardown() {
        let _ = fs::remove_file(TEST_FILE); // Clean up after each test
    }

    #[test]
    fn test_new() {
        setup();
        let result = SkipListFileHandler::new(&TEST_FILE.to_string());
        assert!(result.is_ok());
        teardown();
    }

    #[test]
    fn test_write_and_read_node() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();

        let key = "test_key".to_string();
        let value = Some(vec![1, 2, 3]);
        let node = pager.new_node(key.clone(), value.clone()).unwrap();

        let read_node = pager.read_node(node.id).unwrap();

        assert_eq!(read_node.key, key);
        assert_eq!(read_node.value, value.unwrap());
        teardown();
    }

    #[test]
    fn test_new_node() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();

        let key = "another_test_key".to_string();
        let value = Some(vec![4, 5, 6]);
        let node = pager.new_node(key.clone(), value.clone()).unwrap();

        assert_eq!(node.key, key);
        assert_eq!(node.value, value.unwrap());
        teardown();
    }

    #[test]
    fn test_read_nonexistent_node() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();
        let result = pager.read_node(9999); // Assuming 9999 is an invalid ID
        assert!(result.is_err());
        teardown();
    }

    #[test]
    fn test_delete_node() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();

        let key = "test_key".to_string();
        let value = Some(vec![1, 2, 3]);
        let mut node = pager.new_node(key.clone(), value.clone()).unwrap();

        let read_node = pager.read_node(node.id).unwrap();

        assert_eq!(read_node.key, key);
        assert_eq!(read_node.value, value.unwrap());

        pager
            .delete_node(&mut node, None)
            .expect("err when deleting node:");
        let read_node = pager.read_node(node.id).unwrap();

        assert_eq!(read_node.node_type, NodeType::Deleted);
        assert_eq!(read_node.next_node, None);
        assert_eq!(read_node.value, vec![]);
        assert_eq!("".to_string(), read_node.key);
        assert_eq!(None, read_node.top_node);
        assert_eq!(None, read_node.down_node);
        assert_eq!(None, read_node.prev_node);
        assert_eq!(None, read_node.next_node);
        assert_eq!(NULL_NODE_ID, read_node.id);

        teardown();
    }

    #[test]
    fn update_node() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();

        let key = "test_key".to_string();
        let value = Some(vec![1, 2, 3]);
        let mut node = pager.new_node(key.clone(), value.clone()).unwrap();
        node.next_node = Some(1);
        node.prev_node = Some(2);
        node.down_node = Some(3);
        node.top_node = Some(4);

        let read_node = pager.read_node(node.id).unwrap();

        assert_eq!(read_node.key, key);
        assert_eq!(read_node.value, value.unwrap());

        node.value.extend_from_slice(&[4, 5, 6]);
        pager
            .update_node_value(&mut node)
            .expect("err when updating node:");
        println!("updated node");
        let read_node = pager.read_node(node.id).unwrap();
        println!("read node");
        assert_eq!(read_node.key, key);
        assert_eq!(read_node.value, node.value);
        assert_eq!(read_node.node_type, NodeType::Data);
        assert_eq!(read_node.next_node, Some(1));
        assert_eq!(read_node.prev_node, Some(2));
        assert_eq!(read_node.down_node, Some(3));
        assert_eq!(read_node.top_node, Some(4));

        teardown();
    }
    #[test]
    fn test_header_of_row_node() {
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();
        let node = Node {
            id: 0,
            node_type: NodeType::Linker,
            key: "".to_string(),
            top_node: None,
            down_node: None,
            prev_node: None,
            next_node: None,
            value: vec![],
        };
        let header = pager.convert_node_to_header(&node);
        assert_eq!(header[TYPE_OFFSET], NodeType::Linker as u8);
        assert_eq!(
            0,
            usize::from_be_bytes(
                header[KEY_SIZE_OFFSET..(KEY_SIZE_OFFSET + SIZE_OF_KEY_SIZE)]
                    .try_into()
                    .unwrap()
            )
        )
    }
    #[test]
    fn test_multi_node_read() {
        let mut pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();
        let mut ids = vec![];
        for i in 0..100 {
            ids.push(pager.new_node("".to_string(), None).unwrap().id);
        }
        ids.iter().enumerate().for_each(|(i, &id)| {
            let node = pager.read_node(id).unwrap();
            assert_eq!(node.value, vec![]);
            assert_eq!(node.key, "".to_string());
        })
    }
    #[test]
    fn test_config_file_functions() {
        setup();
        let pager = SkipListFileHandler::new(&TEST_FILE.to_string()).unwrap();
        let rows = vec![1, 2, 3];
        pager.update_config_file(&rows).unwrap();
        let read_rows = pager.read_config_file().unwrap();
        assert_eq!(rows, read_rows);
        let rows = vec![5, 6, 7, 8];
        pager.update_config_file(&rows).unwrap();
        let read_rows = pager.read_config_file().unwrap();
        assert_eq!(rows, read_rows);
        teardown();
    }
}

impl SkipList {
    #[inline]
    fn coin_flip(&self) -> bool {
        rand::random()
    }
    pub fn new(name: &String) -> Res<SkipList> {
        let file_handler = SkipListFileHandler::new(name)?;
        let mut rows = file_handler.read_config_file();
        if let Ok(rows) = rows {
            Ok(SkipList { file_handler, rows })
        } else {
            Ok(SkipList {
                file_handler,
                rows: vec![],
            })
        }
    }

    #[inline]
    fn add_new_row(&mut self, first_row_node: usize) -> Res<usize> {
        let mut row_start = self.file_handler.new_node("".to_string(), None)?;

        row_start.next_node = Some(first_row_node);
        row_start.down_node = self.rows.last().map(|&x| x);
        self.file_handler.update_node_header(&row_start)?;

        self.rows.push(row_start.id);
        self.file_handler.update_config_file(&self.rows)?;
        Ok(row_start.id)
    }

    pub fn insert(&mut self, key: String, value: Vec<usize>) -> Res<()> {
        if let Some(&head_id) = self.rows.last() {
            let mut history = vec![];
            let mut node = self.file_handler.read_node(head_id)?;

            while let Some(down) = node.down_node {
                while let Some(next) = node.next_node {
                    let next_node = self.file_handler.read_node(next)?;
                    if next_node.key > key {
                        break;
                    }
                    node = next_node;
                }
                history.push(node.id);
                node = self.file_handler.read_node(down)?;
            }

            while let Some(next) = node.next_node {
                let next_node = self.file_handler.read_node(next)?;
                if next_node.key > key {
                    break;
                }
                node = next_node;
            }
            if node.key == key {
                // if node exists add the value
                node.value.extend_from_slice(&value);
                self.file_handler.update_node_value(&mut node)?;
            } else {
                let mut new_node = self.file_handler.new_node(key, Some(value))?;

                new_node.next_node = node.next_node;
                new_node.prev_node = Some(node.id);
                node.next_node = Some(new_node.id);

                if let Some(next) = new_node.next_node {
                    let mut next_node = self.file_handler.read_node(next)?; // mybe add a instant write an update prev function
                    next_node.prev_node = Some(new_node.id);
                    self.file_handler.update_node_header(&next_node)?;
                }

                self.file_handler.update_node_header(&node)?;
                self.file_handler.update_node_header(&new_node)?;
                let mut former_node = new_node;

                while self.coin_flip() {
                    if let Some(node_id) = history.pop() {
                        node = self.file_handler.read_node(node_id)?;
                        new_node = self.file_handler.new_node(node.key.clone(), None)?;

                        new_node.down_node = Some(former_node.id);
                        former_node.top_node = Some(new_node.id);

                        new_node.next_node = node.next_node;
                        node.next_node = Some(new_node.id);
                        new_node.prev_node = Some(node_id);

                        if let Some(next) = new_node.next_node {
                            let mut next_node = self.file_handler.read_node(next)?;
                            next_node.prev_node = Some(new_node.id);
                            self.file_handler.update_node_header(&next_node)?;
                        }

                        self.file_handler.update_node_header(&node)?;
                        self.file_handler.update_node_header(&new_node)?;
                        self.file_handler.update_node_header(&former_node)?;
                        former_node = new_node;
                    } else {
                        new_node = self.file_handler.new_node(node.key.clone(), None)?;

                        new_node.down_node = Some(former_node.id);
                        former_node.top_node = Some(new_node.id);

                        let prev = self.add_new_row(new_node.id)?;
                        new_node.prev_node = Some(prev);

                        self.file_handler.update_node_header(&new_node)?;
                        self.file_handler.update_node_header(&former_node)?;

                        former_node = new_node;
                    }
                }
            }
        } else {
            let mut node = self.file_handler.new_node(key, Some(value))?;

            let first_row_start = self.add_new_row(node.id)?;

            node.prev_node = Some(first_row_start);
            self.file_handler.update_node_header(&node)?;

            while self.coin_flip() {
                let mut new_node = self.file_handler.new_node(node.key.clone(), None)?;
                node.top_node = Some(new_node.id);
                new_node.down_node = Some(node.id);

                new_node.prev_node = Some(self.add_new_row(new_node.id)?);

                self.file_handler.update_node_header(&new_node)?;
                self.file_handler.update_node_header(&node)?;

                node = new_node;
            }
        }

        Ok(())
    }
    fn delete_node(&mut self, node: &mut Node) -> Res<()> {
        if let Some(prev) = node.prev_node {
            let mut prev_node = self.file_handler.read_node(prev)?;
            prev_node.next_node = node.next_node;
            self.file_handler.update_node_header(&prev_node)?;
        }
        if let Some(next) = node.next_node {
            let mut next_node = self.file_handler.read_node(next)?;
            next_node.prev_node = node.prev_node;
            self.file_handler.update_node_header(&next_node)?;
        }
        self.file_handler.delete_node(node, None)?;
        Ok(())
    }
    pub fn delete(&mut self, key: &String) -> Res<()> {
        if let Some(&head_id) = self.rows.last() {
            let mut node = self.file_handler.read_node(head_id)?;

            while let Some(down) = node.down_node {
                while let Some(next) = node.next_node {
                    let next_node = self.file_handler.read_node(next)?;
                    if &next_node.key > key {
                        break;
                    }
                    node = next_node;
                }
                node = self.file_handler.read_node(down)?;
            }

            while let Some(next) = node.next_node {
                let next_node = self.file_handler.read_node(next)?;
                if &next_node.key > key {
                    break;
                }
                node = next_node;
            }

            if &node.key == key {
                self.delete_node(&mut node)?;
                while let Some(top) = node.top_node {
                    node = self.file_handler.read_node(top)?;
                    self.delete_node(&mut node)?;
                }
            }
        }
        Ok(())
    }
    pub fn search(&self, key: &String) -> Res<Option<Vec<usize>>> {
        if let Some(&head_id) = self.rows.last() {
            let mut node = self.file_handler.read_node(head_id)?;

            while let Some(down) = node.down_node {
                while let Some(next) = node.next_node {
                    let next_node = self.file_handler.read_node(next)?;
                    if &next_node.key > key {
                        break;
                    }
                    node = next_node;
                }
                node = self.file_handler.read_node(down)?;
            }

            while let Some(next) = node.next_node {
                let next_node = self.file_handler.read_node(next)?;
                if &next_node.key > key {
                    break;
                }
                node = next_node;
            }

            if &node.key == key {
                return Ok(Some(node.value));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod skiplist_tests {
    use super::*;
    use std::fs;

    const TEST_NAME: &str = "test_skiplist";

    fn setup() {
        let _ = fs::remove_file(format!("{}{}", TEST_NAME, SKIP_LIST_CONFIG_FILE_ENDING));
        let _ = fs::remove_file(format!("{}{}", TEST_NAME, SKIP_LIST_MAIN_FILE_ENDING));
    }

    fn teardown() {
        let _ = fs::remove_file(format!("{}{}", TEST_NAME, SKIP_LIST_CONFIG_FILE_ENDING));
        let _ = fs::remove_file(format!("{}{}", TEST_NAME, SKIP_LIST_MAIN_FILE_ENDING));
    }

    #[test]
    fn test_new_skiplist() {
        setup();
        let result = SkipList::new(&TEST_NAME.to_string());
        assert!(result.is_ok());
        teardown();
    }
    #[test]
    fn test_insert() {
        setup();
        let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        let insert_result = skip_list.insert("key1".to_string(), vec![1, 2, 3]);
        assert!(insert_result.is_ok());
        // Optionally, you can add more checks here to verify the state of the skip list
        teardown();
    }
    #[test]
    fn test_delete() {
        setup();
        let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        skip_list.insert("key1".to_string(), vec![1, 2, 3]).unwrap();
        let delete_result = skip_list.delete(&"key1".to_string());
        println!("{:?}", delete_result);
        assert!(delete_result.is_ok());
        // Additional checks can be performed here
        teardown();
    }
    #[test]
    fn test_search() {
        setup();
        let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        skip_list.insert("key1".to_string(), vec![1, 2, 3]).unwrap();
        let search_result = skip_list.search(&"key1".to_string()).unwrap();
        assert_eq!(search_result, Some(vec![1, 2, 3]));
        teardown();
    }
    #[test]
    fn test_coin_flip() {
        setup();
        let skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        let mut res = vec![];
        for i in 0..1000 {
            res.push(skip_list.coin_flip());
        }
        println!("true count {}", res.iter().filter(|x| **x).count());
        assert!(!res.iter().all(|x| *x));
        teardown()
    }
    #[test]
    fn complete_test() {
        setup();
        let range = 100;
        let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        for i in 0..range {
            skip_list
                .insert(i.to_string(), vec![i])
                .expect(&format!("cant insert {}", i));
            println!("inserted {i}");
        }
        for i in 0..range {
            assert_eq!(
                skip_list
                    .search(&i.to_string())
                    .expect(&format!("cant lookup {}", i)),
                Some(vec![i])
            );
            println!("found {i}");
        }
        for i in 0..range {
            println!("updating {i}");
            skip_list
                .insert(i.to_string(), vec![i + 1])
                .expect(&format!("cant insert {}", i));
        }
        for i in 0..range {
            let search_res = skip_list
                .search(&i.to_string())
                .expect(&format!("cant lookup {}", i));
            println!("found {:?}", search_res);
            assert_eq!(search_res, Some(vec![i, i + 1]));
            println!("found updated {i}");
        }
        for i in 0..range {
            skip_list
                .delete(&i.to_string())
                .expect(&format!("cant delete {}", i));
            println!("deleted {i}");
        }
        for i in 0..range {
            assert_eq!(
                skip_list
                    .search(&i.to_string())
                    .expect(&format!("cant lookup {}", i)),
                None
            ); // check if all deleted
            println!("{i} was deleted");
        }
        teardown();
    }
    #[test]
    fn test_read_existing_list() {
        setup();
        let range = 10;
        {
            let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
            for i in 0..range {
                skip_list
                    .insert(i.to_string(), vec![i])
                    .expect(&format!("cant insert {}", i));
            }
        }
        let mut skip_list = SkipList::new(&TEST_NAME.to_string()).unwrap();
        for i in 0..range {
            assert_eq!(
                skip_list
                    .search(&i.to_string())
                    .expect(&format!("cant search {}", i)),
                Some(vec![i])
            );
        }
        teardown();
    }
}
