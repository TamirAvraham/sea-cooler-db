use std::{fs::OpenOptions, path::Path};

use crate::{
    error::{map_err, Error, InternalResult},
    node::{Node, MAX_KEY_SIZE},
    pager::{Pager, HEADER_SIZE, PAGE_SIZE, SIZE_OF_USIZE},
};

pub const MAX_KEYS_IN_NODE: usize =
    (PAGE_SIZE - HEADER_SIZE - SIZE_OF_USIZE) / (MAX_KEY_SIZE + SIZE_OF_USIZE);
#[allow(dead_code)]
pub const LEFT_OVER_BYTES: usize =
    (PAGE_SIZE - HEADER_SIZE - SIZE_OF_USIZE) % (MAX_KEY_SIZE + SIZE_OF_USIZE);
pub const DEFAULT_T: usize = (MAX_KEYS_IN_NODE - 1) / 2;
pub const FILE_ENDING: &str = ".mbpt"; // my b plus tree
pub const VALUES_FILE_ENDING: &str = ".value";
pub const NODES_FILE_ENDING: &str = ".nodes";
pub const DEFAULT_CACHE_SIZE: usize = 100;
pub struct BTreeBuilder {
    path: String,
    t: usize,
}

impl BTreeBuilder {
    pub fn new() -> Self {
        Self {
            path: "".to_string(),
            t: 0,
        }
    }
    pub fn path(mut self, path: String) -> Self {
        self.path = path;
        self
    }
    pub fn t(mut self, t: usize) -> Self {
        self.t = t;
        self
    }

    pub fn build(&self) -> Result<BPlusTree, Error> {
        if (self.path == "") || (self.t > DEFAULT_T || self.t <= 0) {
            return Err(Error::InvalidArguments);
        }

        let mut file_options = OpenOptions::new();
        let pager_file_options = file_options.write(true).create(true).read(true);

        let nodes_file = pager_file_options
            .open(self.path.clone() + NODES_FILE_ENDING + FILE_ENDING)
            .map_err(|e| {
                println!("in nodes file{:?}", e);
                Error::FileError
            })?;

        let values_file = pager_file_options
            .open(self.path.clone() + VALUES_FILE_ENDING + FILE_ENDING)
            .map_err(|e| {
                println!("in values file{:?}", e);
                Error::FileError
            })?;

        let mut pager = Pager::new(nodes_file, values_file, DEFAULT_CACHE_SIZE);
        let root = pager.new_node()?;
        pager.write_node(&root)?;

        Ok(BPlusTree {
            pager,
            t: self.t,
            root_page_id: root.page_id,
        })
    }
}

#[derive(Debug)]
pub struct BPlusTree {
    t: usize,
    pager: Pager,
    root_page_id: usize,
}

impl BPlusTree {
    #[cfg(test)]
    pub fn print(&self) {
        println!("root is: {}\n", self.root_page_id);
        let node = self.pager.read_node(self.root_page_id).unwrap();
        node.print_tree(&self.pager).unwrap()
    }
    #[cfg(test)]
    pub fn to_string(&self) -> String {
        let mut result = format!("root is: {}\n", self.root_page_id);
        let node = self.pager.read_node(self.root_page_id).unwrap();
        result.push_str(&node.tree_to_string(&self.pager).unwrap());
        result
    }
    fn update_internal(
        key: String,
        pager: &mut Pager,
        node_page_id: usize,
        value: String,
    ) -> InternalResult<Option<String>> {
        let mut node = pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key.clone()) {
                    let old_value = pager.read_value(*value_location)?;
                    let old_value = String::from_utf8(old_value)
                        .map_err(map_err(Error::CantReadNode(node_page_id)))?;

                    pager.delete_value(*value_location)?;
                    let new_value_location = pager.new_value(value.as_bytes())?;

                    node.update(key, new_value_location);
                    pager.write_node(&node)?;

                    return Ok(Some(old_value));
                } else {
                    return Ok(None);
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::update_internal(key, pager, *node_page_id, value);
                }
                return Ok(None);
            }
        }
    }
    pub fn update(&mut self,key: String,value: String) ->InternalResult<Option<String>> {
        Self::update_internal(key, &mut self.pager, self.root_page_id, value)
    }

    fn insert_internal(
        pager: &mut Pager,
        key: String,
        value: String,
        t: usize,
        node_page_id: usize,
    ) -> InternalResult<bool> {
        let mut node = pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => {
                let value = pager.new_value(value.as_bytes())?;
                node.insert(key, value);
                return Ok(if node.keys.len() > (2 * t - 1) {
                    node.split(pager, t)?;
                    true
                } else {
                    pager.write_node(&node)?;
                    false
                });
            }
            false => {
                let next_node_page_id = node.get(key.clone()).unwrap().clone();
                if Self::insert_internal(pager, key, value, t, next_node_page_id)? {
                    node = pager.read_node(node_page_id)?;
                    return Ok(if node.keys.len() > (2 * t - 1) {
                        node.split(pager, t)?;
                        true
                    } else {
                        false
                    });
                } else {
                    Ok(false)
                }
            }
        }
    }
    pub fn insert(&mut self, key: String, value: String) -> InternalResult<()> {
        if Self::insert_internal(&mut self.pager, key, value, self.t, self.root_page_id)? {
            let root = self.pager.read_node(self.root_page_id)?;
            self.root_page_id = root.parent_page_id;
        }
        Ok(())
    }
    fn search_internal(
        key: String,
        pager: &Pager,
        node_page_id: usize,
    ) -> InternalResult<Option<String>> {
        let node = pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key) {
                    let value = pager.read_value(*value_location)?;
                    let value = String::from_utf8(value)
                        .map_err(map_err(Error::CantReadNode(node_page_id)))?;
                    return Ok(Some(value));
                } else {
                    return Ok(None);
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::search_internal(key, pager, *node_page_id);
                }
                return Ok(None);
            }
        }
    }
    pub fn search(&self, key: String) -> InternalResult<Option<String>> {
        Self::search_internal(key, &self.pager, self.root_page_id)
    }

    fn delete_node(key: String, pager: &mut Pager, node_page_id: usize) -> Result<(), Error> {
        let mut i = 0;
        let mut node = pager.read_node(node_page_id)?;

        match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key) {
                    pager.delete_value(node.values[i])?;

                    node.keys.remove(i);
                    node.values.remove(i);

                    pager.write_node(&node)?;
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::delete_node(key, pager, *node_page_id);
                }
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn delete(&mut self, key: String) -> Result<(), Error> {
        Self::delete_node(key, &mut self.pager, self.root_page_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::logger::GeneralLogger;
    use std::{
        env,
        fs::{self, File, OpenOptions},
        path::Path,
    };

    use super::*;
    fn cleanup_temp_files() {
        let _ = fs::remove_file("temp.nodes.mbpt");
        let _ = fs::remove_file("temp.value.mbpt");
    }
    const TEST_FILE_NODES: &str = "test_nodes.bin";
    const TEST_FILE_VALUES: &str = "test_values.bin";
    fn create_test_files() -> Result<(File, File), Error> {
        let nodes_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_FILE_NODES)
            .map_err(|_| Error::FileError)?;
        let values_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_FILE_VALUES)
            .map_err(|_| Error::FileError)?;
        Ok((nodes_file, values_file))
    }
    #[test]
    fn test_insert_and_search() {
        let gen_logger_file_path = "defult t btree tests tree log.log";

        let mut oo = OpenOptions::new();
        let mut gen_logger = GeneralLogger::new(
            oo.write(true)
                .read(true)
                .create(true)
                .open(gen_logger_file_path)
                .unwrap(),
            1,
        );

        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new().path(path.clone()).t(2).build().unwrap();

        let range = 113;
        (1..=range).for_each(|i| {
            println!("inserting i:{}", i);
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when inserting i:{}", i);
            tree.insert(key, value).expect(&err_msg);
            tree.print();
            gen_logger
                .info(tree.to_string())
                .expect("cant log info: insert");
        });
        println!("_____________________________________________________________________________________________");
        tree.print();
        println!("_____________________________________________________________________________________________");

        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);

            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value), "{}", err_msg)
        });

        cleanup_temp_files();
        println!("completed tree tests with t=2");
    }

    #[test]
    fn test_delete() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new().path(path.clone()).t(2).build().unwrap();

        let range = 20;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);
            tree.insert(key, value).expect(&err_msg);
            tree.print();
            println!("_____________________________________________________________________________________________");
        });
        tree.print();
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value))
        });
        // Delete a key
        tree.delete("key2".to_string()).unwrap();
        assert_eq!(tree.search("key2".to_string()).unwrap(), None);

        // Attempt to delete a non-existent key
        tree.delete("key4".to_string()).unwrap();
        assert_eq!(tree.search("key4".to_string()).unwrap(), None);
    }

    #[test]
    fn test_update() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new().path(path.clone()).t(2).build().unwrap();

        let range = 100;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);
            tree.insert(key, value).expect(&err_msg);
            tree.print();
            println!("_____________________________________________________________________________________________");
        });
        tree.print();
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value))
        });

        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let old_value = format!("value_{}", i);
            let new_value = format!("value_{}", i+2);
            let err_msg = format!("error when updating i:{}", i);
            let res = tree.update(key.clone(),new_value.clone()).expect(&err_msg);
            assert_eq!(res, Some(old_value));
            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,Some(new_value));
        });
        
    }
    #[test]
    fn test_default_t_tree_just_insert_and_search() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new()
            .path(path.clone())
            .t(DEFAULT_T)
            .build()
            .unwrap();
        let range = DEFAULT_T * 105;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);

            tree.insert(key, value).expect(&err_msg);
            println!("_____________________________________________________________________________________________");
        });
        println!("completed inserting");
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);
            println!("searching {}", key.clone());
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value), "{}", err_msg);
        });

        cleanup_temp_files();
        println!("completed tree tests with t={}", DEFAULT_T);
    }
}
