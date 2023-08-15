use crate::{
    error::{map_err, Error, InternalResult},
    pager::Pager,
};

pub const MAX_KEY_SIZE: usize = 50;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub parent_page_id: usize,
    pub page_id: usize,
    pub keys: Vec<String>,
    pub values: Vec<usize>,
    pub is_leaf: bool,
}

impl Node {
    pub fn get_parent(&self, pager: &Pager) -> InternalResult<Node> {
        pager.read_node(self.parent_page_id)
    }
    pub fn borrow(&mut self, pager: &mut Pager) -> InternalResult<()> {
        let mut parent = self.get_parent(pager)?;

        let location_in_parent = parent
            .values
            .binary_search(&self.page_id)
            .map_err(map_err(Error::BorrowError(self.page_id)))?;
        //is self the biggest node in parent
        if location_in_parent == parent.values.len() - 1 {
            let mut sibling = pager.read_node(parent.values[location_in_parent - 1])?;
            let borrowed_key = if self.is_leaf {
                sibling.keys.pop().unwrap()
            } else {
                parent.keys.last().unwrap().clone()
            };
            let borrowed_value = sibling.values.pop().unwrap();

            self.values.insert(0, borrowed_value);
            self.keys.insert(0, borrowed_key);

            parent.keys[location_in_parent - 1] = if self.is_leaf {
                sibling.keys.last().unwrap().clone()
            } else {
                sibling.keys.pop().unwrap()
            };

            pager.write_node(&sibling)?;
        } else {
            let mut sibling = pager.read_node(parent.values[location_in_parent + 1])?;
            let borrowed_key = if self.is_leaf {
                sibling.keys.remove(0)
            } else {
                parent.keys[location_in_parent].clone()
            };
            let borrowed_value = sibling.values.remove(0);

            self.values.push(borrowed_value);
            self.keys.push(borrowed_key);

            parent.keys[location_in_parent] = if self.is_leaf {
                self.keys.last().unwrap().clone()
            } else {
                sibling.keys.remove(0)
            };
            pager.write_node(&sibling)?;
        }

        pager.write_node(&parent)?;
        pager.write_node(&self)?;
        Ok(())
    }
    pub fn insert(&mut self, key: String, value: usize) {
        let mut i = 0;

        while i < self.keys.len() && self.keys[i] < key {
            i += 1;
        }

        self.values.insert(i, value);
        self.keys.insert(i, key);
    }
    pub fn get(&self, key: String) -> Option<&usize> {
        let mut i = 0;

        while i < self.keys.len() && self.keys[i] < key {
            i += 1;
        }
        if self.is_leaf && self.keys[i]==key {
            return Some(&self.values[i]);
        } else {
            self.values.get(i)
        }
        
    }
    pub fn split(&mut self, pager: &mut Pager, t: usize) -> InternalResult<Node> {
        let new_node_keys = self.keys.split_off(t);
        let new_node_values = self.values.split_off(t);
        let new_node_page_id = pager.new_page()?;

        let mut new_node = Node {
            parent_page_id: self.parent_page_id,
            page_id: new_node_page_id,
            keys: new_node_keys,
            values: new_node_values,
            is_leaf: self.is_leaf,
        };

        let promoted_key = if self.is_leaf {
            self.keys.last().unwrap().clone()
        } else {
            self.keys.pop().unwrap()
        };

        let parent = if self.parent_page_id != 0 {
            let mut parent = pager.read_node(self.parent_page_id)?;
            parent.insert(promoted_key.clone(), new_node_page_id);
            if *parent.keys.last().unwrap() == promoted_key {
                let parent_biggest_key =
                    pager.get_node_max_key(parent.values.last().unwrap().clone())?;

                if parent_biggest_key < *new_node.keys.last().unwrap() {
                    let last_parent_node_index = parent.values.len() - 1;
                    parent
                        .values
                        .swap(last_parent_node_index - 1, last_parent_node_index);
                    //this is the new node's index and the index of the biggset node of the parent
                }
            }
            parent
        } else {
            let parent_page_id = pager.new_page()?;
            let parent = Node {
                parent_page_id: 0,
                page_id: parent_page_id,
                keys: vec![promoted_key],
                values: vec![self.page_id, new_node_page_id],
                is_leaf: false,
            };
            new_node.parent_page_id = parent_page_id;
            self.parent_page_id = parent_page_id;

            parent
        };

        pager.write_node(&self)?;
        pager.write_node(&new_node)?;
        pager.write_node(&parent)?;

        Ok(new_node)
    }
}

impl Node {
    #[cfg(test)]
    pub fn print_tree(&self, pager: &Pager) -> InternalResult<()> {
        self.print_node("", pager)
    }
    #[cfg(test)]
    fn print_node(&self, prefix: &str, pager: &Pager) -> InternalResult<()> {
        println!("{}Node at page: {}", prefix, self.page_id);
        println!("{}Keys: {:?}", prefix, self.keys);
        println!("{}Values: {:?}\n", prefix, self.values);

        if !self.is_leaf {
            for value in &self.values {
                let child_node = pager.read_node(*value)?;
                child_node.print_node(format!("{}   |  ", prefix).as_str(), pager)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
pub fn vec_to_string_vec<T: ToString>(vector: Vec<T>) -> Vec<String> {
    vector.iter().map(ToString::to_string).collect()
}
#[cfg(test)]
mod tests {
    use std::vec;

    use crate::pager::create_pager;

    use super::*;

    #[test]
    fn test_borrow_leaves() {
        let mut pager = create_pager();
        let parent_page_id = pager.new_page().expect("Cannot create parent page");
        let node_page_id = pager.new_page().expect("Cannot create node page");
        let sibling_page_id = pager.new_page().expect("Cannot create sibling page");

        let parent = Node {
            parent_page_id: 0,
            page_id: parent_page_id,
            keys: vec_to_string_vec(vec![4]),
            values: vec![node_page_id, sibling_page_id],
            is_leaf: false,
        };

        let mut node = Node {
            parent_page_id,
            page_id: node_page_id,
            keys: vec_to_string_vec(vec![3, 4]),
            values: vec![3, 4],
            is_leaf: true,
        };

        let sibling = Node {
            parent_page_id,
            page_id: sibling_page_id,
            keys: vec_to_string_vec(vec![5, 6]),
            values: vec![5, 6],
            is_leaf: true,
        };

        pager.write_node(&parent).expect("Cannot write parent node");
        pager.write_node(&node).expect("Cannot write node");
        pager
            .write_node(&sibling)
            .expect("Cannot write sibling node");

        node.borrow(&mut pager).expect("Cannot borrow node");

        let updated_node = pager.read_node(node_page_id).expect("Cannot read node");
        let mut updated_sibling = pager
            .read_node(sibling_page_id)
            .expect("Cannot read sibling");
        let updated_parent = node.get_parent(&pager).expect("cant get updated parent");
        assert_eq!(updated_node, node);
        assert_eq!(updated_node.keys, vec_to_string_vec(vec![3, 4, 5]));
        assert_eq!(updated_sibling.keys, vec_to_string_vec(vec![6]));
        assert_eq!(updated_parent.keys, vec_to_string_vec(vec![5]));

        updated_sibling
            .borrow(&mut pager)
            .expect("sibling cnat borrow");

        let updated_node = pager.read_node(node_page_id).expect("Cannot read node");
        let updated_sibling = pager
            .read_node(sibling_page_id)
            .expect("Cannot read sibling");
        let updated_parent = node.get_parent(&pager).expect("cant get updated parent");

        assert_eq!(updated_node.keys, vec_to_string_vec(vec![3, 4]));
        assert_eq!(updated_sibling.keys, vec_to_string_vec(vec![5, 6]));
        assert_eq!(updated_parent.keys, vec_to_string_vec(vec![4]));
    }

    #[test]
    fn test_borrow_inter() {
        let mut pager = create_pager();
        let parent_page_id = pager.new_page().expect("Cannot create parent page");
        let node_page_id = pager.new_page().expect("Cannot create node page");
        let sibling_page_id = pager.new_page().expect("Cannot create sibling page");

        let parent = Node {
            parent_page_id: 0,
            page_id: parent_page_id,
            keys: vec_to_string_vec(vec![4]),
            values: vec![node_page_id, sibling_page_id],
            is_leaf: false,
        };

        let mut node = Node {
            parent_page_id,
            page_id: node_page_id,
            keys: vec_to_string_vec(vec![2, 3]),
            values: vec![2, 3, 4],
            is_leaf: false,
        };

        let sibling = Node {
            parent_page_id,
            page_id: sibling_page_id,
            keys: vec_to_string_vec(vec![5, 6]),
            values: vec![5, 6, 7],
            is_leaf: false,
        };

        pager.write_node(&parent).expect("Cannot write parent node");
        pager.write_node(&node).expect("Cannot write node");
        pager
            .write_node(&sibling)
            .expect("Cannot write sibling node");

        node.borrow(&mut pager).expect("Cannot borrow node");

        let updated_node = pager.read_node(node.page_id).expect("Cannot read node");
        let mut updated_sibling = pager
            .read_node(sibling_page_id)
            .expect("Cannot read sibling");
        let updated_parent = node.get_parent(&pager).expect("cant get updated parent");
        assert_eq!(node, updated_node);
        assert_eq!(updated_node.values, vec![2, 3, 4, 5]);
        assert_eq!(updated_node.keys, vec_to_string_vec(vec![2, 3, 4]));
        assert_eq!(updated_sibling.values, vec![6, 7]);
        assert_eq!(updated_sibling.keys, vec_to_string_vec(vec![6]));
        assert_eq!(updated_parent.keys, vec_to_string_vec(vec![5]));

        updated_sibling
            .borrow(&mut pager)
            .expect("sibling cnat borrow");

        let updated_node = pager.read_node(node_page_id).expect("Cannot read node");
        let updated_sibling = pager
            .read_node(sibling_page_id)
            .expect("Cannot read sibling");
        let updated_parent = node.get_parent(&pager).expect("cant get updated parent");

        assert_eq!(updated_node.values, vec![2, 3, 4]);
        assert_eq!(updated_node.keys, vec_to_string_vec(vec![2, 3]));
        assert_eq!(updated_sibling.values, vec![5, 6, 7]);
        assert_eq!(updated_sibling.keys, vec_to_string_vec(vec![5, 6]));
        assert_eq!(updated_parent.keys, vec_to_string_vec(vec![4]));
    }

    #[test]
    fn test_node_insert() {
        let mut pager = create_pager();
        let mut node = Node {
            parent_page_id: 0,
            page_id: pager.new_page().expect("cant create page for error"),
            keys: vec![],
            values: vec![],
            is_leaf: true,
        };

        node.insert("5".to_string(), 1);
        node.insert("6".to_string(), 4);
        node.insert("3".to_string(), 2);
        node.insert("===".to_string(), 6);

        assert_eq!(node.values, vec![2, 1, 4, 6]);
    }
    #[test]
    fn test_split_with_no_parent() {
        let mut pager = create_pager();
        let t = 2;
        let node_page_id = pager.new_page().expect("cant create page for new node");

        let mut node = Node {
            parent_page_id: 0,
            page_id: node_page_id,
            keys: vec_to_string_vec(vec![1, 2, 3]),
            values: vec![1, 2, 3],
            is_leaf: true,
        };
        let new_node = node.split(&mut pager, t).expect("cant split node");
        let node = pager.read_node(node_page_id).expect("cant read node");
        let parent = pager
            .read_node(node.parent_page_id)
            .expect("cant get parent");

        assert_eq!(node.keys, vec_to_string_vec(vec![1, 2]));
        assert_eq!(node.values, vec![1, 2]);
        assert!(node.is_leaf);

        assert!(new_node.is_leaf);
        assert_eq!(new_node.keys, vec_to_string_vec(vec![3]));
        assert_eq!(new_node.values, vec![3]);

        assert_eq!(node.parent_page_id, new_node.parent_page_id);

        assert_eq!(parent.keys, vec_to_string_vec(vec![2]));
        assert_eq!(parent.values, vec![node.page_id, new_node.page_id]);
        assert!(!parent.is_leaf)
    }

    #[test]
    fn test_split_with_parent() {
        let mut pager = create_pager();
        let t = 2;
        let node_page_id = pager.new_page().expect("cant create page for new node");

        let mut node = Node {
            parent_page_id: 0,
            page_id: node_page_id,
            keys: vec_to_string_vec(vec![1, 2, 3]),
            values: vec![1, 2, 3],
            is_leaf: true,
        };

        let mut new_node = node.split(&mut pager, t).expect("cant split node");
        let _parent = pager
            .read_node(new_node.parent_page_id)
            .expect("cnat read parent from pager");

        new_node.insert("5".to_string(), 8);
        new_node.insert("6".to_string(), 8);
        new_node.insert("7".to_string(), 8);
        new_node.insert("8".to_string(), 8);

        let mut new_node = new_node.split(&mut pager, t).expect("cnat split new node");

        let parent = pager
            .read_node(new_node.parent_page_id)
            .expect("cnat read parent from pager");
        let prev_node = pager
            .read_node(parent.values[1])
            .expect("cnat get prev node from file");

        assert_eq!(prev_node.keys, vec_to_string_vec(vec![3, 5]));
        assert_eq!(new_node.keys, vec_to_string_vec(vec![6, 7, 8]));
        assert_eq!(parent.keys, vec_to_string_vec(vec![2, 5]));

        new_node.insert("9".to_string(), 0);
        new_node.insert("91".to_string(), 0);

        let new_node = new_node.split(&mut pager, t).expect("cant split new node");
        let prev_node = pager
            .read_node(parent.values[2])
            .expect("cnat get prev node from file");

        let mut parent = pager
            .read_node(new_node.parent_page_id)
            .expect("cnat read parent from pager");

        assert_eq!(parent.keys, vec_to_string_vec(vec![2, 5, 7]));
        assert_eq!(parent.values.len(), 4);

        let new_inter = parent.split(&mut pager, t).expect("cant split parent");
        assert_eq!(new_inter.keys, vec_to_string_vec(vec![7]));
        assert_eq!(parent.keys, vec_to_string_vec(vec![2]));
        assert_eq!(parent.values.len(), 2);
        assert_eq!(new_inter.values.len(), 2);
        let new_parent = pager
            .read_node(new_inter.parent_page_id)
            .expect("cant split parent");
        assert_eq!(new_parent.keys, vec_to_string_vec(vec![5]));
        new_parent.print_tree(&pager).expect("cnat print tree");
        let i = 0;
    }
}
