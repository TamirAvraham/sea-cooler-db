#[derive(Debug)]
pub struct Node<F> {
    nodes: Vec<Node<F>>,
    key: String,
    handler: Option<F>,
    is_wild_card: bool,
}

impl<F> Node<F> {
    pub fn new(key: &str) -> Self {
        Node {
            nodes: Vec::new(),
            key: key.to_string(),
            handler: None,
            is_wild_card: key.starts_with('{') && key.ends_with('}'),
        }
    }
    pub fn insert(&mut self, path: &str, handler: F) {
        match path.split_once('/') {
            Some((root, "")) => {
                self.handler = Some(handler);
                self.key = root.to_string();
            }
            Some(("", path)) => self.insert(path, handler),
            Some((root, path)) => {
                let node = self
                    .nodes
                    .iter_mut()
                    .find(|n| n.key == root || n.is_wild_card);
                match node {
                    None => {
                        let mut node = Node::new(path);
                        node.handler = Some(handler);
                        self.nodes.push(node);
                    }
                    Some(n) => n.insert(path, handler),
                }
            }
            None => {
                let mut node = Node::new(path);
                node.handler = Some(handler);
                self.nodes.push(node);
            }
        }
    }
    pub fn get(&self, path: &str) -> Option<&F> {
        match path.split_once('/') {
            Some((root, "")) => {
                if self.key == root || self.is_wild_card {
                    self.handler.as_ref()
                } else {
                    None
                }
            }
            Some(("", path)) => self.get(path),
            Some((root, path)) => {
                let node = self.nodes.iter().find(|n| n.key == root || n.is_wild_card);
                match node {
                    None => None,
                    Some(n) => n.get(path),
                }
            }
            None => {
                let node = self.nodes.iter().find(|n| n.key == path || n.is_wild_card);
                match node {
                    None => None,
                    Some(n) => n.handler.as_ref(),
                }
            }
        }
    }
}
pub type RadixTree<F> = Node<F>;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_insert() {
        let mut tree = RadixTree::new("/");
        tree.insert("/hello", "world");
        tree.insert("/hello/world", "hello");
        tree.insert("/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/beni", "beni");
    }
    #[test]
    fn test_get() {
        let mut tree = RadixTree::new("/");
        tree.insert("/hello", "world");
        tree.insert("/hello/world", "hello");
        tree.insert("/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/hello/world/hello", "world");
        tree.insert("/hello/world/hello/world/hello/world/hello/world", "hello");
        tree.insert("/hello/world/hello/world/beni", "beni");
        assert_eq!(tree.get("/hello").unwrap(), &"world");
        assert_eq!(tree.get("/hello/world").unwrap(), &"hello");
        assert_eq!(tree.get("/hello/world/hello").unwrap(), &"world");
        assert_eq!(tree.get("/hello/world/hello/world").unwrap(), &"hello");
        assert_eq!(
            tree.get("/hello/world/hello/world/hello").unwrap(),
            &"world"
        );
        assert_eq!(
            tree.get("/hello/world/hello/world/hello/world").unwrap(),
            &"hello"
        );
        assert_eq!(
            tree.get("/hello/world/hello/world/hello/world/hello")
                .unwrap(),
            &"world"
        );
        assert_eq!(
            tree.get("/hello/world/hello/world/hello/world/hello/world")
                .unwrap(),
            &"hello"
        );
        assert_eq!(tree.get("/hello/world/hello/world/beni").unwrap(), &"beni");
    }
}
