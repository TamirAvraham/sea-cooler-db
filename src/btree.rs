use std::{path::Path, fs::OpenOptions};

use crate::{pager::{Pager, PAGE_SIZE, SIZE_OF_USIZE, HEADER_SIZE}, node::{MAX_KEY_SIZE, Node}, error::{Error, InternalResult, map_err}};


pub const MAX_KEYS_IN_NODE:usize=(PAGE_SIZE-HEADER_SIZE-SIZE_OF_USIZE)/(MAX_KEY_SIZE+SIZE_OF_USIZE);
#[allow(dead_code)]
pub const LEFT_OVER_BYTES:usize=(PAGE_SIZE-HEADER_SIZE-SIZE_OF_USIZE)%(MAX_KEY_SIZE+SIZE_OF_USIZE);
pub const DEFAULT_T:usize=(MAX_KEYS_IN_NODE-1)/2;
pub const FILE_ENDING: &str = ".mbpt"; // my b plus tree
pub const VALUE_FILE_ENDING: &str = ".value";
pub const NODES_FILE_ENDING: &str = ".nodes";
pub const DEFAULT_CACHE_SIZE:usize=100;
pub struct BTreeBulider {
    path: String,
    t: usize,
}

impl BTreeBulider {
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
            .open(self.path.clone() + VALUE_FILE_ENDING + FILE_ENDING)
            .map_err(|e| {
                println!("in values file{:?}", e);
                Error::FileError
            })?;

        let mut pager = Pager::new(nodes_file, values_file,DEFAULT_CACHE_SIZE);
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
    t:usize,
    pager:Pager,
    root_page_id:usize
}

impl BPlusTree {
    #[cfg(test)]
    pub fn print(&self){
        println!("root is: {}",self.root_page_id);
        let node=self.pager.read_node(self.root_page_id).unwrap();
        node.print_tree(&self.pager).unwrap()
    }
    fn insert_internal(pager:&mut Pager,key:String,value:String,t:usize,node_page_id:usize)->InternalResult<bool>{
        let mut node=pager.read_node(node_page_id)?;
        match node.is_leaf{
            true => {
                let value=pager.new_value(value.as_bytes())?;
                node.insert(key, value);
                return Ok(if node.keys.len()>(2*t-1) {
                    node.split(pager, t)?;
                    true
                } else {
                    pager.write_node(&node)?;
                    false
                })
                
            },
            false => {
                let next_node_page_id=node.get(key.clone()).unwrap().clone();
                if Self::insert_internal(pager, key, value, t, next_node_page_id)?{
                    node=pager.read_node(node_page_id)?;
                    return Ok(if node.keys.len()>(2*t-1) {
                        node.split(pager, t)?;
                        true
                    } else {
                        false
                    })
                } else {
                    Ok(false)
                }
            },
        }
        
    }
    pub fn insert(&mut self,key:String,value:String)->InternalResult<()>{
        if Self::insert_internal(&mut self.pager, key, value, self.t, self.root_page_id)? {
            let root=self.pager.read_node(self.root_page_id)?;
            self.root_page_id=root.parent_page_id;
        }
        Ok(())
    }
    fn search_internal(key:String,pager:&Pager,node_page_id:usize)->InternalResult<Option<String>>{
        let node=pager.read_node(node_page_id)?;
        match node.is_leaf {
            true => {
                if let Some(value_location) = node.get(key) {
                    let value=pager.read_value(*value_location)?;
                    let value=String::from_utf8(value).map_err(map_err(Error::CantReadNode(node_page_id)))?;
                    return Ok(Some(value));
                } else{
                    return Ok(None);
                }
                
            },
            false => {
                if let Some(node_page_id) = node.get(key.clone()) {
                    return Self::search_internal(key, pager, *node_page_id)
                }
                return Ok(None);
            },
        }
    }
    pub fn search(&self,key:String)->InternalResult<Option<String>>{
        Self::search_internal(key, &self.pager, self.root_page_id)
    }

    fn delete_node(
        key: String,
        pager: &mut Pager,
        node_page_id:usize,
    ) -> Result<(), Error> {
        let mut i = 0;
        let mut node=pager.read_node(node_page_id)?;

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
                    return Self::delete_node(key, pager, *node_page_id)
                }
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn delete(&mut self, key: String) -> Result<(), Error> {
        Self::delete_node(
            key,
            &mut self.pager,
            self.root_page_id,
        )
    }
}


#[cfg(test)]
mod tests{
    use std::{path::Path, fs::{OpenOptions, self, File}, env};
    use crate::pager::delete_file;

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
        let path="temp".to_string();
        let mut tree=BTreeBulider::new().path(path.clone()).t(2).build().unwrap();
        let range=500;
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);

            tree.insert(key, value).expect(&err_msg);
            tree.print();
            println!("_____________________________________________________________________________________________");
        });
        (1..=range).for_each(|i| {
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when serching for i:{}",i);

            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,Some(value),"{}", err_msg)
        });
        
        cleanup_temp_files();
        println!("completed tree tests with t=2");
        let mut tree=BTreeBulider::new().path(path.clone()).t(5).build().unwrap();
        (1..=range).for_each(|i| {
            println!("inserting i:{}",i);
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when inserting i:{}",i);

            tree.insert(key, value).expect(&err_msg);
            tree.print();
        });

        (1..=range).for_each(|i| {
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when serching for i:{}",i);

            let res=tree.search(key).expect(&err_msg);
            tree.print();
            assert_eq!(res,Some(value))
        });

        let res=tree.search("bonzo".to_string()).expect("fuck");
        assert_eq!(res,None);

        cleanup_temp_files();


    }

    #[test]
    fn test_delete() {
        let path="temp".to_string();
        let mut tree=BTreeBulider::new().path(path.clone()).t(2).build().unwrap();

        let range=5;
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
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when serching for i:{}",i);
            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,Some(value))
        });

        // Delete a key
        tree.delete("key2".to_string()).unwrap();
        assert_eq!(tree.search("key2".to_string()).unwrap(), None);

        // Attempt to delete a non-existent key
        tree.delete("key4".to_string()).unwrap();
        assert_eq!(tree.search("key4".to_string()).unwrap(), None);
    }
    #[test]
    fn test_default_t_tree_just_insert_and_search() {
        let path="temp".to_string();
        let mut tree=BTreeBulider::new().path(path.clone()).t(DEFAULT_T).build().unwrap();
        let range=DEFAULT_T*100;
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
            let key=format!("key_{}",i);
            let value=format!("value_{}",i);
            let err_msg=format!("error when serching for i:{}",i);
            println!("searching {}",key.clone());
            let res=tree.search(key).expect(&err_msg);
            assert_eq!(res,Some(value),"{}", err_msg);

        });
        
        cleanup_temp_files();
        println!("completed tree tests with t=34");
    }
}