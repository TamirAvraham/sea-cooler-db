use std::{cell::RefCell, fs::File};

const PAGE_SIZE: usize = 1024 * 4;
const SIZE_OF_USIZE: usize = size_of::<usize>();
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

type PagerFile=RefCell<File>

struct Pager {
    nodes_file:PagerFile,
    values_file:PagerFile,
    max_page_id:usize
}



impl Pager {
    #[inline]
    fn file_to_pager_file(file:File) -> PagerFile {
        RefCell::new(file)
    }

    pub fn 
}