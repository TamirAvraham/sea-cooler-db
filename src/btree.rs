use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, Write},
    path::Path,
};
use std::collections::{HashMap, HashSet};
use std::panic::set_hook;

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
    name: String,
}

impl BTreeBuilder {
    pub fn new() -> Self {
        Self {
            path: "".to_string(),
            t: 0,
            name: "".to_string(),
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
    pub fn name(mut self, name: &String) -> Self {
        self.name = name.clone();
        self
    }
    fn read_root_id_from_file(&self) -> Result<usize, Error> {
        let mut options = OpenOptions::new();

        let mut file = options
            .read(true)
            .open(format!("{}{}", self.name, FILE_ENDING))
            .map_err(|e| Error::FileError)?;
        let mut root_id_bytes = [0; SIZE_OF_USIZE];
        file.read_exact(&mut root_id_bytes)
            .map_err(|_| Error::FileError)?;

        Ok(usize::from_be_bytes(root_id_bytes))
    }
    pub fn build(self) -> Result<BPlusTree, Error> {
        if (self.path == "") || (self.t > DEFAULT_T || self.t <= 0) || (self.name == "") {
            return Err(Error::InvalidArguments);
        }

        let mut file_options = OpenOptions::new();
        file_options.write(true).create(true).read(true);

        let nodes_file = file_options
            .open(self.path.clone() + NODES_FILE_ENDING + FILE_ENDING)
            .map_err(|e| {
                println!("in nodes file{:?}", e);
                Error::FileError
            })?;
        let values_path =self.path.clone() + VALUES_FILE_ENDING + FILE_ENDING;
        let values_file = if Path::new(&values_path).exists(){

            let mut file_options = OpenOptions::new();
            file_options.read(true).write(true);
            file_options
                .open(values_path)
                .map_err(|e| {
                    println!("in values file{:?}", e);
                    Error::FileError
                })?
        } else {
            let mut file_options = OpenOptions::new();
            file_options.write(true).create(true).read(true);
            file_options
                .open(values_path)
                .map_err(|e| {
                    println!("in values file{:?}", e);
                    Error::FileError
                })?
        };

        let mut pager = Pager::new(nodes_file, values_file, DEFAULT_CACHE_SIZE);

        let root_page_id = if let Ok(root_id) = self.read_root_id_from_file() {
            root_id
        } else {
            let root = pager.new_node()?;
            pager.write_node(&root)?;
            root.page_id
        };

        let ret = Ok(BPlusTree {
            pager,
            t: self.t,
            root_page_id,
            name: self.name,
        });
        ret.as_ref().unwrap().save_info_on_file()?;
        ret
    }
}

#[derive(Debug)]
pub struct BPlusTree {
    name: String,
    t: usize,
    pub pager: Pager,
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
    fn save_info_on_file(&self) -> Result<(), Error> {
        let path = format!("{}{}", self.name, FILE_ENDING);

        let mut open_options = OpenOptions::new();
        let mut file = open_options
            .create(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                println!("{}", e);
                Error::FileError
            })?;

        file.seek(std::io::SeekFrom::Start(0)).map_err(|e| {
            println!("{}", e);
            Error::FileError
        })?;

        file.write_all(&self.root_page_id.to_be_bytes())
            .map_err(|e| {
                println!("{}", e);
                Error::FileError
            })
    }
    /// # Description
    /// function updates a node in the tree
    /// # Arguments
    ///
    /// * `key`: key to update
    /// * `pager`: pager of the tree
    /// * `node_page_id`: current node page id
    /// * `value`: new value
    ///
    /// returns: Result<Option<Vec<u8, Global>>, Error> (old value)
    ///

    fn update_internal(
        key: String,
        pager: &mut Pager,
        node_page_id: usize,
        value: &[u8],
    ) -> InternalResult<Option<(Vec<u8>,usize)>> {
        let mut node = pager.read_node(node_page_id)?;
        return match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key.clone()) {
                    let old_value = pager.read_value(*value_location)?;
                    pager.delete_value(*value_location)?;
                    let new_value_location = pager.new_value(value)?;

                    node.update(key, new_value_location);
                    pager.write_node(&node)?;

                    Ok(Some((old_value,new_value_location)))
                } else {
                    Ok(None)
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::update_internal(key, pager, *node_page_id, value);
                }
                Ok(None)
            }
        }
    }
    /// #  Description
    ///  function is a wrapper for update internal
    /// # Arguments
    ///
    /// * `key`: key to update
    /// * `value`: new value
    ///
    /// returns: Result<Option<Vec<u8, Global>>, Error> (old value)
    pub fn update(&mut self, key: String, value: &[u8]) -> InternalResult<Option<(Vec<u8>,usize)>> {
        Self::update_internal(key, &mut self.pager, self.root_page_id, value)
    }

    /// #  Description
    ///  function inserts a value into the tree
    /// # Arguments
    ///
    /// * `pager`: pager of the tree
    /// * `key`: key of new value
    /// * `value`: value to insert
    /// * `t`: t of the tree
    /// * `node_page_id`: current node page id
    ///
    /// returns: Result<(bool, usize), Error> (did split?,value location)

    fn insert_internal(
        pager: &mut Pager,
        key: String,
        value: &[u8],
        t: usize,
        node_page_id: usize,
    ) -> InternalResult<(bool, usize)> {
        let mut node = pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => {
                let value = pager.new_value(value)?;
                node.insert(key, value);
                return Ok(if node.keys.len() > (2 * t - 1) {
                    node.split(pager, t)?;
                    (true, value)
                } else {
                    pager.write_node(&node)?;
                    (false, value)
                });
            }
            false => {
                let next_node_page_id = node.get(key.clone()).unwrap().clone();
                let (insert_internal, value_location) =
                    Self::insert_internal(pager, key, value, t, next_node_page_id)?;
                if insert_internal {
                    node = pager.read_node(node_page_id)?;
                    return Ok(if node.keys.len() > (2 * t - 1) {
                        node.split(pager, t)?;
                        (true, value_location)
                    } else {
                        (false, value_location)
                    });
                } else {
                    Ok((false, value_location))
                }
            }
        }
    }
    /// #  Description
    ///  function is a wrapper for insert internal
    /// # Arguments
    ///
    /// * `key`: key of new value
    /// * `value`: value to insert
    ///
    /// returns: Result<usize, Error> (value location)
    pub fn insert(&mut self, key: String, value: &[u8]) -> InternalResult<usize> {
        let (insert_internal, ret) =
            Self::insert_internal(&mut self.pager, key, value, self.t, self.root_page_id)?;
        if insert_internal {
            let root = self.pager.read_node(self.root_page_id)?;
            self.root_page_id = root.parent_page_id;
            self.save_info_on_file()?;
        }
        Ok(ret)
    }
    /// #  Description
    /// function looks for the value location of a key
    /// # Arguments
    ///
    /// * `key`: key to look for
    /// * `pager`: pager of the tree
    /// * `node_page_id`: current node page id
    ///
    /// returns: Result<Option<usize>, Error> (value location)
    fn search_for_value_pointer(
        key: String,
        pager: &Pager,
        node_page_id: usize,
    ) -> InternalResult<Option<(String,usize)>> {
        let node = pager.read_node(node_page_id)?;
        return match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key.clone()) {
                    Ok(Some((key, *value_location)))
                } else {
                    Ok(None)
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::search_for_value_pointer(key, pager, *node_page_id);
                }
                Ok(None)
            }
        }
    }
    /// #  Description
    /// function looks for the value of a key
    /// # Arguments
    ///
    /// * `key`: key to look for
    /// * `pager`: pager of the tree
    /// * `node_page_id`: current node page id
    ///
    /// returns: Result<Option<Vec<u8>>, Error> (value)
    fn search_internal(
        key: String,
        pager: &Pager,
        node_page_id: usize,
    ) -> InternalResult<Option<Vec<u8>>> {
        let node = pager.read_node(node_page_id)?;
        return match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key) {
                    let value = pager.read_value(*value_location)?;
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::search_internal(key, pager, *node_page_id);
                }
                Ok(None)
            }
        }
    }
    /// #  Description
    /// function looks for a node that contains key
    /// # Arguments
    ///
    /// * `key`: key to look for
    /// * `pager`: pager of the tree
    /// * `node_page_id`: current node page id
    /// * `root_is_a_leaf`: is root node a leaf
    /// returns: Result<Option<usize>, Error> (node page id)
    fn search_node_by_key(
        key: String,
        pager: &Pager,
        node_page_id: usize,
        root_is_a_leaf:bool
    ) -> InternalResult<Option<usize>> {
        let node = pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => Ok(if let Some(ret) = node.get(key) {
                Some(if root_is_a_leaf { node_page_id } else { 0 })
            } else {
                None
            }),
            false => {
                if let Some(found_node_page_id) = node.get(key.clone()) {
                    return Ok(if let Some(result) = Self::search_node_by_key(key, pager, *found_node_page_id,root_is_a_leaf)? {
                        Some(if result==0 { *found_node_page_id } else { result })
                    }else {
                        None
                    });
                }
                return Ok(None);
            }
        }
    }
    /// #  Description
    ///  function gets all values in a node in a range
    /// # Arguments
    ///
    /// * `start`: start of range
    /// * `end`: end of range
    /// * `pager`: pager of the tree
    /// * `node_page_id`: page id of current node
    ///
    /// returns: Result<Vec<(String, usize), Global>, Error> ((key,value_location))
    fn get_values(
        start: &String,
        end: &String,
        pager: &Pager,
        node_page_id: usize,
    ) -> InternalResult<Vec<(String, usize)>> {
        let node = pager.read_node(node_page_id)?;
        Ok(match node.is_leaf {
            true => node
                .keys
                .into_iter()
                .zip(node.values.into_iter())
                .filter(|(key, _)| start <= key && key <= end)
                .collect(),
            false => {
                let mut ret = vec![];

                if let Some(last_key) = node.keys.last() {
                    if last_key <= end {
                        if let Some(&node_page_id) =
                            node.values.last()
                        {
                            ret.extend(Self::get_values(start, end, pager, node_page_id)?);
                        }
                    }
                }

                for (_, v) in node
                    .keys
                    .into_iter()
                    .zip(node.values.into_iter())
                    .filter(|(key, _)| start <= key && key <= end)
                {
                    ret.extend(Self::get_values(start, end, pager, v)?);
                }

                ret
            }
        })
    }
    /// #  Description
    /// function finds the first common node between 2 nodes
    /// # Arguments
    ///
    /// * `start`: start of range
    /// * `end`: end of range
    /// * `pager`: pager of the tree
    /// * `root_page_id`: page id of root node
    /// * `root_is_a_leaf`: is root node a leaf
    ///
    /// returns: Result<usize, Error> (node page id)
    fn find_nodes_intersection(
        start: String,
        end: String,
        root_page_id: usize,
        root_is_a_leaf:bool,
        pager: &Pager,
    ) -> InternalResult<usize> {
        let start_node_id =
            Self::search_node_by_key(start, pager, root_page_id,root_is_a_leaf)?.ok_or(Error::CantGetValue)?;
        let mut start_node = pager.read_node(start_node_id)?;

        let end_node_id =
            Self::search_node_by_key(end, pager, root_page_id,root_is_a_leaf)?.ok_or(Error::CantGetValue)?;
        let mut end_node = pager.read_node(end_node_id)?;
        if start_node_id==end_node_id {
            return Ok(start_node_id);
        }
        while start_node.parent_page_id != end_node.parent_page_id {
            start_node = start_node.get_parent(pager)?;

            end_node = start_node.get_parent(pager)?;
        }

        Ok(start_node.parent_page_id)
    }
    /// #  Description
    /// function searches for a range in the tree
    /// # Arguments
    ///
    /// * `start`: start of range
    /// * `end`: end of range
    /// * `pager`: pager of the tree
    /// * `root_page_id`: id of the root page
    /// * `root_is_a_leaf`: is the root node a leaf
    ///
    /// returns: Result<HashSet<(String, usize)>, Error> ((key,value_location))
    fn range_search_internal(
        start: String,
        end: String,
        root_page_id: usize,
        root_is_a_leaf:bool,
        pager: &Pager,
    ) -> InternalResult<HashSet<(String, usize)>> {
        let intersection =
            match Self::find_nodes_intersection(start.clone(), end.clone(), root_page_id,root_is_a_leaf,pager) {
                Ok(ret) => Ok(ret),
                Err(err) => if err==Error::CantGetValue {
                    return Ok(HashSet::new())
                }else {
                    Err(err)
                }
            }?;
        Ok(Self::get_values(&start, &end, pager, intersection)?.into_iter()
            .collect::<HashSet<(String,usize)>>())
    }
    /// #   Description
    /// function is a wrapper for range_search_internal.
    /// function searches for a range of values in the tree and returns a vector of its keys and value pointers
    /// # Arguments
    ///
    /// * `start`: start of the range
    /// * `end`: end of the range
    ///
    /// returns: Result<Vec<(String, usize), Global>, Error>
    pub fn range_search(&self, start: String, end: String) -> Result<Vec<(String, usize)>, Error> {
        if start==end {
            return Ok(match Self::search_for_value_pointer(start, &self.pager,self.root_page_id)? {
                None => {vec![]}
                Some(ret) => {vec![ret]}
            });
        }
        let is_root_a_leaf={
            let root=self.pager.read_node(self.root_page_id)?;
            root.is_leaf
        };
        Ok(Self::range_search_internal(start, end, self.root_page_id,is_root_a_leaf, &self.pager)?.into_iter().collect::<Vec<(String, usize)>>())
    }
    /// # Description
    /// function is a wrapper for search_internal.
    /// function searches for a value in the tree
    /// # Arguments
    ///
    /// * `key`: key to look for
    ///
    /// returns: Result<Option<Vec<u8, Global>>, Error>

    pub fn search(&self, key: String) -> InternalResult<Option<Vec<u8>>> {
        Self::search_internal(key, &self.pager, self.root_page_id)
    }

    /// # Description
    ///  function deletes a node from the tree
    /// # Arguments
    ///
    /// * `key`: key to delete
    /// * `pager`: pager of the tree
    /// * `node_page_id`: page id of the node current node
    ///
    /// returns: Result<(), Error>
    ///
    fn delete_node(
        key: String,
        pager: &mut Pager,
        node_page_id:usize,
    ) -> Result<(), Error> {
        let mut node=pager.read_node(node_page_id)?;

        match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key.clone()) {
                    let i = node.keys.iter().position(|r| r.clone() == key.clone()).unwrap();

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
    /// # Description
    ///  function deletes a value from the tree
    /// # Arguments
    /// * `key`: key to delete
    ///
    /// returns: Result<(), Error>
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
            tree.insert(key, value.as_bytes()).expect(&err_msg);
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
            assert_eq!(res, Some(value.as_bytes().to_vec()), "{}", err_msg);
        });
        println!("completed search");
        (1..=range).rev().for_each(|i|{
            let key=format!("key_{}",i);
            let err_msg=format!("error when serching for i:{}",i);
            println!("deleting {}",key.clone());
            tree.delete(key.clone()).expect(&err_msg);
            (1..i).for_each(|i| {
                let key=format!("key_{}",i);
                let value=format!("value_{}",i);
                let err_msg=format!("error when serching for i:{}",i);
                println!("searching {}",key.clone());
                let res=tree.search(key).expect(&err_msg);

                assert_eq!(res,Some(value.as_bytes().to_vec()),"{}", err_msg);
            });
            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,None,"{}",err_msg);
        });
        println!("delete complete");



        cleanup_temp_files();
        println!("completed tree tests with t=2");
    }

    #[test]
    fn test_delete() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new()
            .path(path.clone())
            .name(&path)
            .t(2)
            .build()
            .unwrap();

        let range = 100;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);
            tree.insert(key, value.as_bytes()).expect(&err_msg);
            tree.print();
            println!("_____________________________________________________________________________________________");
        });
        tree.print();
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value.as_bytes().to_vec()), "{}", err_msg);
        });
        // Delete a key
        tree.delete("key_2".to_string()).unwrap();
        assert_eq!(tree.search("key_2".to_string()).unwrap(), None);

        println!("After deleting an existing key:");
        tree.print();
        // Attempt to delete a non-existent key
        tree.delete("key_4".to_string()).unwrap();
        assert_eq!(tree.search("key_4".to_string()).unwrap(), None);

        println!("After deleting non existent key:");
        tree.print();
    }

    #[test]
    fn test_update() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new()
            .path(path.clone())
            .t(2)
            .name(&path)
            .build()
            .unwrap();

        let range = 100;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);
            tree.insert(key, value.as_bytes()).expect(&err_msg);
            tree.print();
            println!("_____________________________________________________________________________________________");
        });
        tree.print();
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when searching for i:{}", i);
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value.as_bytes().to_vec()), "{}", err_msg);
        });

        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let old_value = format!("value_{}", i);
            let new_value = format!("value_{}", i + 2);
            let err_msg = format!("error when updating i:{}", i);
            let res = tree
                .update(key.clone(), new_value.as_bytes())
                .expect(&err_msg);
            assert_eq!(res.unwrap().0, old_value.as_bytes().to_vec());
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(new_value.as_bytes().to_vec()), "{}", err_msg);
        });
    }

    
    #[test]
    fn test_default_t_tree_just_insert_and_search() {
        let path="temp".to_string();
        let mut tree=BTreeBuilder::new().name(&path.clone()).path(path).t(DEFAULT_T).build().unwrap();
        let range=DEFAULT_T*100;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);

            tree.insert(key, value.as_bytes()).expect(&err_msg);
            println!("_____________________________________________________________________________________________");
        });
        println!("completed inserting");
        (1..=range).for_each(|i| {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let err_msg = format!("error when serching for i:{}", i);
            println!("searching {}", key.clone());
            let res = tree.search(key).expect(&err_msg);
            assert_eq!(res, Some(value.as_bytes().to_vec()), "{}", err_msg);
        });
        println!("completed search");
        (1..=range).rev().for_each(|i|{
            let key=format!("key_{}",i);
            let err_msg=format!("error when serching for i:{}",i);
            println!("deleting {}",key.clone());
            tree.delete(key.clone()).expect(&err_msg);
            (1..i).for_each(|i| {
                let key=format!("key_{}",i);
                let value=format!("value_{}",i);
                let err_msg=format!("error when serching for i:{}",i);
                println!("searching {}",key.clone());
                let res=tree.search(key).expect(&err_msg);
                assert_eq!(res,Some(value.as_bytes().to_vec()),"{}", err_msg);
            });
            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,None,"{}",err_msg);
        });
        println!("delete complete");

        cleanup_temp_files();
        println!("completed tree tests with t={}", DEFAULT_T);
    }
    #[test]
    fn test_range_search_basic() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new()
            .path(path.clone())
            .t(2)
            .name(&path)
            .build()
            .unwrap();

        // Insert test data
        for i in 1..=10 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            tree.insert(key, value.as_bytes()).unwrap();
        }

        // Perform a range search
        let start_key = "key_3".to_string();
        let end_key = "key_7".to_string();
        let results = tree.range_search(start_key, end_key).unwrap();

        // Check results
        let expected_keys: Vec<String> = (3..=7).map(|i| format!("key_{}", i)).collect();
        let expected_values: Vec<usize> = (3..=7).map(|i| i).collect();
        assert_eq!(results.len(), expected_values.len(),"results {:?}, expected {:?}",results,expected_keys);
        assert!(results.iter().all(|(x,_)| expected_keys.contains(x)))
    }

    #[test]
    fn test_range_search_edge_cases() {
        let path = "temp".to_string();
        let mut tree = BTreeBuilder::new()
            .path(path.clone())
            .t(2)
            .name(&path)
            .build()
            .unwrap();

        // Insert test data
        for i in 1..=10 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            tree.insert(key, value.as_bytes()).unwrap();
        }

        // Test empty range
        let results = tree
            .range_search("key_15".to_string(), "key_20".to_string())
            .unwrap();
        assert!(results.is_empty());

        // Test range with single element
        let results = tree
            .range_search("key_5".to_string(), "key_5".to_string())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "key_5");
    }
}
