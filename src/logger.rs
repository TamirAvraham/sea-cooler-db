use std::{
    fs::File,
    io::{self, stdout, Read, Seek, Write},
    mem::size_of,
    sync::{RwLock, RwLockWriteGuard},
};

use crate::pager::SIZE_OF_USIZE;
const ID_SIZE: usize = SIZE_OF_USIZE;
const ID_OFFSET: usize = ID_SIZE;
const LOG_TYPE_SIZE: usize = size_of::<u8>();
const LOG_TYPE_OFFSET: usize = LOG_TYPE_SIZE + ID_OFFSET;
const COMPLETED_SIZE: usize = 1;
const COMPLETED_OFFSET: usize = LOG_TYPE_OFFSET + COMPLETED_SIZE;
const TRY_COUNTER_SIZE: usize = size_of::<u8>();
const TRY_COUNTER_OFFSET: usize = COMPLETED_OFFSET + TRY_COUNTER_SIZE;
const PARAM_LEN_SIZE: usize = SIZE_OF_USIZE;
enum OperationLoggerError {
    InvalidOperationCode(u8),
    InvalidLogLocation(usize),
    CantReadId,
    CantReadCompleted,
    CantReadTryCounter,
    CantReadOpType,
    CantReadParam,
}
enum OperationType {
    Insert(String, String),
    Select(String),
    Delete(String),
    Update(String, String),
}
struct OperationLog {
    completed: bool,
    id: usize,
    start_location: usize,
    try_counter: usize,
    op_type: OperationType,
}
struct OperationLogger {
    log_file: RwLock<File>,
}

impl OperationType {
    pub fn to_u8(&self) -> u8 {
        match self {
            OperationType::Insert(_, _) => 0x01,
            OperationType::Select(_) => 0x02,
            OperationType::Delete(_) => 0x03,
            OperationType::Update(_, _) => 0x4,
        }
    }
}

//[ID - 8b][completed - 1b][try_counter - 8b][type - 1b][params - ?b]
impl OperationLogger {
    fn get_string_from_file(
        file: &mut RwLockWriteGuard<'_, File>,
    ) -> Result<String, OperationLoggerError> {
        let mut size_bytes = [0u8; SIZE_OF_USIZE];
        file.read_exact(&mut size_bytes)
            .map_err(|_| OperationLoggerError::CantReadParam)?;

        let mut data = vec![0u8; usize::from_be_bytes(size_bytes)];

        file.read_exact(&mut data)
            .map_err(|_| OperationLoggerError::CantReadParam)?;

        String::from_utf8(data).map_err(|_| OperationLoggerError::CantReadParam)
    }
    fn get_op_type(
        file: &mut RwLockWriteGuard<'_, File>,
        op_type_byte: u8,
    ) -> Result<OperationType, OperationLoggerError> {
        match op_type_byte {
            0x01 => Ok(OperationType::Insert(
                Self::get_string_from_file(file)?,
                Self::get_string_from_file(file)?,
            )),
            0x02 => Ok(OperationType::Select(Self::get_string_from_file(file)?)),
            0x03 => Ok(OperationType::Delete(Self::get_string_from_file(file)?)),
            0x04 => Ok(OperationType::Update(
                Self::get_string_from_file(file)?,
                Self::get_string_from_file(file)?,
            )),
            _ => Err(OperationLoggerError::InvalidOperationCode(op_type_byte)),
        }
    }

    fn read_log(&self, offset: &usize) -> Result<OperationLog, OperationLoggerError> {
        let mut file = self.log_file.write().unwrap();
        file.seek(io::SeekFrom::Start(*offset as u64))
            .map_err(|_| OperationLoggerError::InvalidLogLocation(*offset))?;

        let mut usize_buff = [0u8; SIZE_OF_USIZE];
        file.read_exact(&mut usize_buff)
            .map_err(|_| OperationLoggerError::CantReadId)?;

        let id = usize::from_be_bytes(usize_buff);

        let mut byte_arr = [0u8];
        file.read_exact(&mut byte_arr)
            .map_err(|_| OperationLoggerError::CantReadCompleted)?;

        let completed = match byte_arr[0] {
            0x1 => Ok(true),
            0x0 => Ok(false),
            _ => Err(OperationLoggerError::CantReadCompleted),
        }?;

        file.read_exact(&mut usize_buff)
            .map_err(|_| OperationLoggerError::CantReadTryCounter)?;

        let try_counter = usize::from_be_bytes(usize_buff);

        file.read_exact(&mut byte_arr)
            .map_err(|_| OperationLoggerError::CantReadOpType)?;

        let op_type=Self::get_op_type(&mut file, byte_arr[0])?;

        Ok(OperationLog{ completed, id, start_location: offset.clone(), try_counter, op_type })
    }
}
