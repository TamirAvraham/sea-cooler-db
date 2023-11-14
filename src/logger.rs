use std::{
    fs::File,
    io::{self, stdout, Read, Seek, Write},
    mem::size_of,
    sync::{RwLock, RwLockWriteGuard}, cell::Cell,
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
#[derive(Debug,PartialEq, Eq, PartialOrd, Ord)]
enum OperationLoggerError {
    InvalidOperationCode(u8),
    InvalidLogLocation(usize),

    CantReadId,
    CantReadCompleted,
    CantReadTryCounter,
    CantReadOpType,
    CantReadParam,

    CantWriteId,
    CantWriteCompleted,
    CantWriteTryCounter,
    CantWriteOpType,
    CantWriteParam,

    CantMarkLogAsComplete
}
#[derive(Debug,PartialEq, Eq, PartialOrd, Ord)]
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
    id:Cell<usize>,
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
    fn write_string_to_file(
        file: &mut RwLockWriteGuard<'_, File>,
        string: &String,
    ) -> Result<(), OperationLoggerError> {
        file.write_all(&string.len().to_be_bytes())
            .map_err(|_| OperationLoggerError::CantWriteParam)?;
        file.write_all(string.as_bytes())
            .map_err(|_| OperationLoggerError::CantWriteParam)?;
        Ok(())
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

    fn read_log_from_file(&self, offset: &usize) -> Result<OperationLog, OperationLoggerError> {
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

        let op_type = Self::get_op_type(&mut file, byte_arr[0])?;

        Ok(OperationLog {
            completed,
            id,
            start_location: offset.clone(),
            try_counter,
            op_type,
        })
    }
    fn write_op_type(
        file: &mut RwLockWriteGuard<'_, File>,
        op_type: &OperationType,
    ) -> Result<(), OperationLoggerError> {
        file.write_all(&[op_type.to_u8()])
            .map_err(|_| OperationLoggerError::CantWriteOpType)?;
        match op_type {
            OperationType::Select(key) => Self::write_string_to_file(file, key),
            OperationType::Delete(key) => Self::write_string_to_file(file, key),
            OperationType::Insert(key, value) => {
                Self::write_string_to_file(file, key)?;
                Self::write_string_to_file(file, value)
            }
            OperationType::Update(key, value) => {
                Self::write_string_to_file(file, key)?;
                Self::write_string_to_file(file, value)
            }
        }
    }
    fn write_log(&mut self, log: &OperationLog) -> Result<usize, OperationLoggerError> {
        let mut file = self.log_file.write().unwrap();
        let ret = file.metadata().unwrap().len();

        file.seek(io::SeekFrom::End(0))
            .map_err(|_| OperationLoggerError::CantWriteId)?;
        file.write_all(&log.id.to_be_bytes())
            .map_err(|_| OperationLoggerError::CantWriteId)?;

        file.write_all(&[match log.completed {
            true => 0x01,
            false => 0x0,
        }])
        .map_err(|_| OperationLoggerError::CantWriteCompleted)?;

        file.write_all(&log.try_counter.to_be_bytes())
            .map_err(|_| OperationLoggerError::CantWriteTryCounter)?;
        Self::write_op_type(&mut file, &log.op_type)?;

        Ok(ret as usize)
    }

    pub fn log_operation(&mut self,op_type: OperationType)->Result<OperationLog,OperationLoggerError>{
        let mut ret=OperationLog{ completed: false, id: self.id.get(), start_location: 0, try_counter: 0, op_type };
        let start_location=self.write_log(&ret)?;
        ret.start_location=start_location;
        self.id.set(self.id.get()+1);

        Ok(ret)
    }
    pub fn read_log(&self,start_location:&usize)->Result<OperationLog,OperationLoggerError>{
        self.read_log_from_file(start_location)
    }
    pub fn set_log_as_completed(&mut self,log_start:&usize)->Result<(),OperationLoggerError>{
        let mut file=self.log_file.write().unwrap();
        file.seek(io::SeekFrom::Start((*log_start+ID_SIZE) as u64)).map_err(|_| OperationLoggerError::CantMarkLogAsComplete)?;
        file.write(&[0x1]).map_err(|_| OperationLoggerError::CantMarkLogAsComplete)?;

        Ok(())
    }
    pub fn get_id(&self)->usize{
        self.id.get()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{remove_file, OpenOptions};

    const TEST_FILE_PATH: &str = "test_log.txt";

    #[test]
    fn log_operation_should_create_log_and_return_it() {
        // Arrange
        let _ = remove_file(TEST_FILE_PATH); // Ensure the file doesn't exist before the test
        let mut logger = OperationLogger {
            log_file: RwLock::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(TEST_FILE_PATH)
                    .unwrap(),
            ),
            id: Cell::new(1),
        };

        // Act
        let log = logger.log_operation(OperationType::Insert("key".to_string(), "value".to_string())).unwrap();

        // Assert
        assert_eq!(log.id, 1);
        assert_eq!(log.completed, false);
        assert_eq!(log.try_counter, 0);
        assert_eq!(log.start_location, 0); // As this is the first log, it should be at the start
    }

    #[test]
    fn read_log_should_return_log_from_file() {
        // Arrange
        let _ = remove_file(TEST_FILE_PATH); // Ensure the file doesn't exist before the test
        let mut logger = OperationLogger {
            log_file: RwLock::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(TEST_FILE_PATH)
                    .unwrap(),
            ),
            id: Cell::new(1),
        };
        let log = logger.log_operation(OperationType::Insert("key".to_string(), "value".to_string())).unwrap();

        // Act
        let read_log = logger.read_log(&log.start_location).unwrap();

        // Assert
        assert_eq!(read_log.id, log.id);
        assert_eq!(read_log.completed, log.completed);
        assert_eq!(read_log.try_counter, log.try_counter);
        assert_eq!(read_log.start_location, log.start_location);
        assert_eq!(read_log.op_type, log.op_type);
    }

    #[test]
    fn set_log_as_completed_should_mark_log_as_completed() {
        // Arrange
        let _ = remove_file(TEST_FILE_PATH); // Ensure the file doesn't exist before the test
        let mut logger = OperationLogger {
            log_file: RwLock::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(TEST_FILE_PATH)
                    .unwrap(),
            ),
            id: Cell::new(1),
        };
        let log = logger.log_operation(OperationType::Insert("key".to_string(), "value".to_string())).unwrap();

        // Act
        logger.set_log_as_completed(&log.start_location).unwrap();
        let mut file = logger.log_file.write().unwrap();
        let mut completed_byte = [0u8];
        file.seek(io::SeekFrom::Start(log.start_location as u64 + ID_SIZE as u64)).unwrap();
        file.read_exact(&mut completed_byte).unwrap();

        // Assert
        assert_eq!(completed_byte[0], 0x01); // Should be marked as completed
    }
}
