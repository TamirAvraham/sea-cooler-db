use std::fmt::Debug;

#[derive(Debug,PartialEq, Eq, PartialOrd, Ord)]
pub enum Error {
    CantSeekToPage(usize),
    CantGetNodesFileForWrite,
    CantGetValuesFileForWrite,
    CantSeekToValue(usize),
    CantWritePage,
    CantWriteValue,
    CantReadValue,
    CantReadNode(usize),
    CantWriteNode(usize),
    CantWriteCacheToDisk((usize,usize)),
    CantGetValue,
    BorrowError(usize),
    MergeError(usize),
    InvalidArguments,
    FileError,
    ParentError,
    MovingCacheError(usize),
    CantDeletePage(usize),
    CantDeleteNode(usize),

}

pub type InternalResult<T>=Result<T,Error>;




pub fn map_err<E: std::fmt::Debug>(new_error:Error)->impl FnOnce(E) -> Error{
    move |e|{
        println!("error :{:?}",e);
        new_error
    }
}