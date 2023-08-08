pub const MAX_KEY_SIZE:usize=50;
#[derive(Debug,PartialEq, Eq, PartialOrd, Ord)]
pub struct Node {
    pub parent_page_id:usize,
    pub page_id:usize,
    pub keys:Vec<String>,
    pub values:Vec<usize>,
    pub is_leaf:bool,
}