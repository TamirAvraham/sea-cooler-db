use std::{
    fs::{self, File, OpenOptions},
    io::{self, stdout, Read, Seek, Write},
    mem::size_of,
    path::Path,
    sync::{RwLock, RwLockWriteGuard},
};

use crate::{
    btree::{BPlusTree, FILE_ENDING, NODES_FILE_ENDING, VALUES_FILE_ENDING},
    helpers::copy_file,
    pager::SIZE_OF_USIZE,
};
const ID_SIZE: usize = SIZE_OF_USIZE;
const ID_OFFSET: usize = ID_SIZE;
const LOG_TYPE_SIZE: usize = size_of::<u8>();
const LOG_TYPE_OFFSET: usize = LOG_TYPE_SIZE + ID_OFFSET;
const COMPLETED_SIZE: usize = 1;
const COMPLETED_OFFSET: usize = LOG_TYPE_OFFSET + COMPLETED_SIZE;
const TRY_COUNTER_SIZE: usize = SIZE_OF_USIZE;
const TRY_COUNTER_OFFSET: usize = COMPLETED_OFFSET + TRY_COUNTER_SIZE;
const PARAM_LEN_SIZE: usize = SIZE_OF_USIZE;
const FAIL_LOG_PATH: &str = "fail log.flog";

pub const RESTORER_DIR: &str = "backup";
pub const RESTORER_SETTINGS_FILE_ENDING: &str = ".restorer.config";
const RESTORER_RECOMMENDED_DIFF: usize = 30;
const MAX_TRY_COUNTER: usize = 5;

pub const LOGGER_CONFIG_FILENAME: &str = "logger.config";
pub const OPERATION_LOGGER_FILE_ENDING: &str = ".oplogger";
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LoggerError {
    //OP logger
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

    CantMarkLogAsComplete,
    CantFindLog(usize),
    CantAddOperationToFailLog,
    CantIncrementTryCounter,
    CantRestoreFilesFromBackup,
    //Gen logger
    CanWriteToOutput,

    //Restorer
    CantCreateBackupDir,
    CantLoadLastId,
    CantLoadLastOpStart,
    CantLoadBackupFiles,
    CantUpdateLog,

    //Logger
    CantWriteToConfigFile,
    CantReadFromConfigFile,
}

pub enum LogType {
    Info,
    Warning,
    Error,
}
impl<T: Write> Default for GeneralLogger<T> {
    fn default() -> Self {
        Self {
            output: None,
            id: 1,
        }
    }
}
impl<T> GeneralLogger<T>
where
    T: Write,
{
    pub fn new(output: T, id: usize) -> GeneralLogger<T> {
        GeneralLogger {
            output: Some(output),
            id,
        }
    }
    /// #  Description
    /// function writes message into log
    /// # Arguments
    ///
    /// * `log_type`: what log write the message to
    /// * `message`: message that will be written
    ///
    /// returns: Result<(), Error>
    pub fn log(&mut self, log_type: LogType, message: String) -> Result<(), LoggerError> {
        let log = format!(
            "{}| {}: {}.\n",
            self.id,
            match log_type {
                LogType::Info => "Info",
                LogType::Warning => "Warning",
                LogType::Error => "Error",
            },
            message
        );

        if let Some(output) = &mut self.output {
            output
                .write_all(log.as_bytes())
                .map_err(|_| LoggerError::CanWriteToOutput)?;
        } else {
            stdout()
                .lock()
                .write_all(log.as_bytes())
                .map_err(|_| LoggerError::CanWriteToOutput)?;
        }

        self.id += 1;
        Ok(())
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
    pub fn info(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Info, message)
    }
    pub fn warning(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Warning, message)
    }
    pub fn error(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Error, message)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum OperationType {
    Insert(String, Vec<u8>),
    Select(String),
    Delete(String),
    Update(String, Vec<u8>),
}

#[derive(Debug)]
pub struct OperationLog {
    completed: bool,
    id: usize,
    start_location: usize,
    try_counter: usize,
    op_type: OperationType,
}
struct OperationLogger {
    log_file: RwLock<File>,
    id: usize,
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

impl ToString for OperationType {
    fn to_string(&self) -> String {
        match self {
            OperationType::Insert(k, v) => format!("Insert {}:{:?}", k, v),
            OperationType::Select(k) => format!("Select {}", k),
            OperationType::Delete(k) => format!("Delete {}", k),
            OperationType::Update(k, v) => format!("Update {}:{:?}", k, v),
        }
    }
}

//[ID - 8b][completed - 1b][try_counter - 8b][type - 1b][params - ?b]
impl OperationLogger {
    pub fn new(log_file: File, id: usize) -> Self {
        Self {
            log_file: RwLock::new(log_file),
            id,
        }
    }
    /// #  Description
    /// function writes operation into fail log
    /// # Arguments
    ///
    /// * `op_type`: type of operation to write into fail log
    ///
    /// returns: Result<(), Error>
    fn write_to_fail_log(&self, op_type: &OperationType) -> Result<(), LoggerError> {
        let mut open_options = OpenOptions::new();
        Self::write_string_to_file(
            &mut RwLock::new(
                open_options
                    .write(true)
                    .create(true)
                    .open(FAIL_LOG_PATH)
                    .map_err(|_| LoggerError::CantAddOperationToFailLog)?,
            )
            .write()
            .unwrap(),
            &op_type.to_string(),
        )
    }
    fn get_vec_u8_from_file(file: &mut RwLockWriteGuard<'_, File>) -> Result<Vec<u8>, LoggerError> {
        let mut size_bytes = [0u8; SIZE_OF_USIZE];
        file.read_exact(&mut size_bytes)
            .map_err(|_| LoggerError::CantReadParam)?;

        let mut data = vec![0u8; usize::from_be_bytes(size_bytes)];

        file.read_exact(&mut data)
            .map_err(|_| LoggerError::CantReadParam)?;
        Ok(data)
    }
    fn get_string_from_file(file: &mut RwLockWriteGuard<'_, File>) -> Result<String, LoggerError> {
        String::from_utf8(Self::get_vec_u8_from_file(file)?).map_err(|_| LoggerError::CantReadParam)
    }
    fn write_bytes_to_file(
        file: &mut RwLockWriteGuard<'_, File>,
        bytes: &Vec<u8>,
    ) -> Result<(), LoggerError> {
        file.write_all(&bytes.len().to_be_bytes())
            .map_err(|_| LoggerError::CantWriteParam)?;
        file.write_all(bytes)
            .map_err(|_| LoggerError::CantWriteParam)?;
        Ok(())
    }
    fn write_string_to_file(
        file: &mut RwLockWriteGuard<'_, File>,
        string: &String,
    ) -> Result<(), LoggerError> {
        file.write_all(&string.len().to_be_bytes())
            .map_err(|_| LoggerError::CantWriteParam)?;
        file.write_all(string.as_bytes())
            .map_err(|_| LoggerError::CantWriteParam)?;
        Ok(())
    }
    /// #  Description
    /// function turns byte into operation type
    /// # Arguments
    ///
    /// * `file`: name of file
    /// * `op_type_byte`: byte to turn into operation type
    ///
    /// returns: Result<OperationType, Error>
    fn get_op_type(
        file: &mut RwLockWriteGuard<'_, File>,
        op_type_byte: u8,
    ) -> Result<OperationType, LoggerError> {
        match op_type_byte {
            0x01 => Ok(OperationType::Insert(
                Self::get_string_from_file(file)?,
                Self::get_vec_u8_from_file(file)?,
            )),
            0x02 => Ok(OperationType::Select(Self::get_string_from_file(file)?)),
            0x03 => Ok(OperationType::Delete(Self::get_string_from_file(file)?)),
            0x04 => Ok(OperationType::Update(
                Self::get_string_from_file(file)?,
                Self::get_vec_u8_from_file(file)?,
            )),
            _ => Err(LoggerError::InvalidOperationCode(op_type_byte)),
        }
    }
    /// #  Description
    /// function calculates size of operation log
    /// # Arguments
    ///
    /// * `log`: type of log
    ///
    /// returns: Result<usize, Error> (size of log)
    fn calc_op_log_size(log: &OperationType) -> usize {
        ID_SIZE
            + COMPLETED_SIZE
            + TRY_COUNTER_SIZE
            + LOG_TYPE_SIZE
            + match log {
                OperationType::Insert(k, v) => k.len() + v.len() + SIZE_OF_USIZE * 2,
                OperationType::Select(k) => k.len() + SIZE_OF_USIZE,
                OperationType::Delete(k) => k.len() + SIZE_OF_USIZE,
                OperationType::Update(k, v) => k.len() + v.len() + SIZE_OF_USIZE * 2,
            }
    }
    /// #  Description
    /// function reads file and returns log found in it
    /// # Arguments
    ///
    /// * `offset`: position to start reading from in the file
    ///
    /// returns: Result<OperationLog, Error>
    fn read_log_from_file(&self, offset: &usize) -> Result<OperationLog, LoggerError> {
        let mut file = self.log_file.write().unwrap();
        file.seek(io::SeekFrom::Start(*offset as u64))
            .map_err(|_| LoggerError::InvalidLogLocation(*offset))?;

        let mut usize_buff = [0u8; SIZE_OF_USIZE];
        file.read_exact(&mut usize_buff)
            .map_err(|_| LoggerError::CantReadId)?;

        let id = usize::from_be_bytes(usize_buff);

        let mut byte_arr = [0u8];
        file.read_exact(&mut byte_arr)
            .map_err(|_| LoggerError::CantReadCompleted)?;

        let completed = match byte_arr[0] {
            0x1 => Ok(true),
            0x0 => Ok(false),
            _ => Err(LoggerError::CantReadCompleted),
        }?;

        file.read_exact(&mut usize_buff)
            .map_err(|_| LoggerError::CantReadTryCounter)?;

        let try_counter = usize::from_be_bytes(usize_buff);

        file.read_exact(&mut byte_arr)
            .map_err(|_| LoggerError::CantReadOpType)?;

        let op_type = Self::get_op_type(&mut file, byte_arr[0])?;

        Ok(OperationLog {
            completed,
            id,
            start_location: offset.clone(),
            try_counter,
            op_type,
        })
    }
    /// #  Description
    /// function writes operation into file
    /// # Arguments
    ///
    /// * `file`: file to write into
    /// * `op_type`: type of operation being written
    ///
    /// returns: Result<(), Error>
    fn write_op_type(
        file: &mut RwLockWriteGuard<'_, File>,
        op_type: &OperationType,
    ) -> Result<(), LoggerError> {
        file.write_all(&[op_type.to_u8()])
            .map_err(|_| LoggerError::CantWriteOpType)?;
        match op_type {
            OperationType::Select(key) => Self::write_string_to_file(file, key),
            OperationType::Delete(key) => Self::write_string_to_file(file, key),
            OperationType::Insert(key, value) => {
                Self::write_string_to_file(file, key)?;
                Self::write_bytes_to_file(file, value)
            }
            OperationType::Update(key, value) => {
                Self::write_string_to_file(file, key)?;
                Self::write_bytes_to_file(file, value)
            }
        }
    }
    /// #  Description
    /// function writes log into file
    /// # Arguments
    ///
    /// * `log`: log to write into
    ///
    /// returns: Result<usize, Error> (length of log)
    fn write_log(&mut self, log: &OperationLog) -> Result<usize, LoggerError> {
        let mut file = self.log_file.write().unwrap();
        let ret = file.metadata().unwrap().len();

        file.seek(io::SeekFrom::End(0))
            .map_err(|_| LoggerError::CantWriteId)?;
        file.write_all(&log.id.to_be_bytes())
            .map_err(|_| LoggerError::CantWriteId)?;

        file.write_all(&[match log.completed {
            true => 0x01,
            false => 0x0,
        }])
        .map_err(|_| LoggerError::CantWriteCompleted)?;

        file.write_all(&log.try_counter.to_be_bytes())
            .map_err(|_| LoggerError::CantWriteTryCounter)?;
        Self::write_op_type(&mut file, &log.op_type)?;

        Ok(ret as usize)
    }
    /// #  Description
    /// function writes operation into file
    /// # Arguments
    ///
    /// * `op_type`: type of operation to write
    ///
    /// returns: Result<OperationLog, Error>
    pub fn log_operation(&mut self, op_type: OperationType) -> Result<OperationLog, LoggerError> {
        let mut ret = OperationLog {
            completed: false,
            id: self.id,
            start_location: 0,
            try_counter: 0,
            op_type,
        };
        let start_location = self.write_log(&ret)?;
        ret.start_location = start_location;
        self.id += 1;

        Ok(ret)
    }
    pub fn read_log(&self, start_location: &usize) -> Result<OperationLog, LoggerError> {
        self.read_log_from_file(start_location)
    }
    pub fn mark_log_as_completed(&mut self, log_start: &usize) -> Result<(), LoggerError> {
        let mut file = self.log_file.write().unwrap();
        file.seek(io::SeekFrom::Start((*log_start + ID_SIZE) as u64))
            .map_err(|_| LoggerError::CantMarkLogAsComplete)?;
        file.write_all(&[0x1])
            .map_err(|_| LoggerError::CantMarkLogAsComplete)?;

        Ok(())
    }
    pub fn increment_log_try_counter(&mut self, log: &OperationLog) -> Result<(), LoggerError> {
        let mut file = self.log_file.write().unwrap();
        file.seek(io::SeekFrom::Start(
            (log.start_location + COMPLETED_SIZE + ID_SIZE) as u64,
        ))
        .map_err(|_| LoggerError::CantIncrementTryCounter)?;
        file.write_all(&(log.try_counter + 1).to_be_bytes())
            .map_err(|_| LoggerError::CantIncrementTryCounter)
    }

    pub fn get_id(&self) -> usize {
        self.id
    }
    /// #  Description
    /// function returns last completed operation
    /// # Arguments
    ///
    /// * `start_offset`: where to start reading from
    ///
    /// returns: Result<OperationLog, Error> (last operation)
    pub fn get_last_completed_operation(
        &self,
        start_offset: &usize,
    ) -> Result<OperationLog, LoggerError> {
        let mut ret = None;
        let mut offset = *start_offset;

        while let Ok(log) = self.read_log(&offset) {
            if !log.completed {
                break;
            }
            offset = log.start_location + Self::calc_op_log_size(&log.op_type);
            ret = Some(log);
        }
        if let Some(ret) = ret {
            Ok(ret)
        } else {
            Err(LoggerError::CantFindLog(*start_offset))
        }
    }
    pub fn get_all_logs_from_start(&mut self, start_offset: &usize) -> Vec<OperationLog> {
        let mut ret = vec![];
        let mut offset = *start_offset;

        while let Ok(log) = self.read_log(&offset) {
            offset = log.start_location + Self::calc_op_log_size(&log.op_type);
            ret.push(log);
        }

        ret
    }
    pub fn get_all_logs(&mut self) -> Vec<OperationLog> {
        self.get_all_logs_from_start(&0)
    }

}
pub struct GeneralLogger<T: Write> {
    output: Option<T>,
    id: usize,
}

struct Restorer {
    name: String,
    last_op_id: usize,
    last_op_start: usize,
}
impl Restorer {
    fn get_restorer_path(name: &String) -> String {
        format!("{}_{}", name, RESTORER_DIR)
    }
    pub fn load(name: String) -> Result<Self, LoggerError> {
        let restorer_path_as_string = Self::get_restorer_path(&name);
        let nodes_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, name, VALUES_FILE_ENDING, FILE_ENDING
        );
        let values_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, name, NODES_FILE_ENDING, FILE_ENDING
        );
        let config_path = format!(
            "{}\\{}{}",
            restorer_path_as_string, name, RESTORER_SETTINGS_FILE_ENDING
        );

        let restorer_path = Path::new(&restorer_path_as_string);

        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);

        if !(restorer_path.exists() && restorer_path.is_dir()) {
            fs::create_dir(restorer_path).map_err(|_| LoggerError::CantCreateBackupDir)?;
            copy_file(
                &format!("{}{}{}", name, VALUES_FILE_ENDING, FILE_ENDING),
                &nodes_path,
            )
            .map_err(|_| LoggerError::CantCreateBackupDir)?;
            copy_file(
                &format!("{}{}{}", name, NODES_FILE_ENDING, FILE_ENDING),
                &values_path,
            )
            .map_err(|_| LoggerError::CantCreateBackupDir)?;
            let mut config_file = options
                .open(&config_path)
                .map_err(|_| LoggerError::CantCreateBackupDir)?;
            config_file
                .write_all(&(0usize).to_be_bytes())
                .map_err(|_| LoggerError::CantCreateBackupDir)?;
            config_file
                .write_all(&(0usize).to_be_bytes())
                .map_err(|_| LoggerError::CantCreateBackupDir)?;
        }

        let mut config_file = options
            .open(&config_path)
            .map_err(|_| LoggerError::CantLoadLastId)?;

        let mut last_id = [0u8; SIZE_OF_USIZE];
        config_file
            .read_exact(&mut last_id)
            .map_err(|_| LoggerError::CantLoadLastId)?;

        let mut last_op_start = [0u8; SIZE_OF_USIZE];
        config_file
            .read_exact(&mut last_op_start)
            .map_err(|_| LoggerError::CantLoadLastOpStart)?;

        Ok(Self {
            name: name.clone(),
            last_op_start: usize::from_be_bytes(last_op_start),
            last_op_id: usize::from_be_bytes(last_id),
        })
    }

    /// #  Description
    /// function updates last completed operation
    /// # Arguments
    ///
    /// * `op_logger`: previous last completed operation
    ///
    /// returns: Result<(), Error>
    pub fn update(&mut self, op_logger: &OperationLogger) -> Result<(), LoggerError> {
        let new_last_completed_log = op_logger.get_last_completed_operation(&self.last_op_start)?;
        let restorer_path_as_string = Self::get_restorer_path(&self.name);
        let nodes_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, self.name, VALUES_FILE_ENDING, FILE_ENDING
        );
        let values_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, self.name, NODES_FILE_ENDING, FILE_ENDING
        );
        let config_path = format!(
            "{}\\{}{}",
            restorer_path_as_string, self.name, RESTORER_SETTINGS_FILE_ENDING
        );

        fs::write(
            &config_path,
            new_last_completed_log
                .id
                .to_be_bytes()
                .iter()
                .cloned()
                .chain(
                    new_last_completed_log
                        .start_location
                        .to_be_bytes()
                        .iter()
                        .cloned(),
                )
                .collect::<Vec<u8>>(),
        )
        .map_err(|_| LoggerError::CantUpdateLog)?;

        copy_file(
            &format!("{}{}{}", self.name, VALUES_FILE_ENDING, FILE_ENDING),
            &nodes_path,
        )
        .map_err(|_| LoggerError::CantUpdateLog)?;
        copy_file(
            &format!("{}{}{}", self.name, NODES_FILE_ENDING, FILE_ENDING),
            &values_path,
        )
        .map_err(|_| LoggerError::CantUpdateLog)?;

        self.last_op_id = new_last_completed_log.id;
        self.last_op_start = new_last_completed_log.start_location;

        Ok(())
    }
    pub fn should_update(&self, current_op_max_id: usize) -> bool {
        self.last_op_id + RESTORER_RECOMMENDED_DIFF <= current_op_max_id
    }
    pub fn get_last_backed_operation_id(&self) -> usize {
        self.last_op_id
    }
    pub fn get_last_backed_operation_start(&self) -> usize {
        self.last_op_start
    }
    /// #  Description
    /// function returns all uncompleted operations
    /// # Arguments
    ///
    /// * `op_logger`: log to get operations from
    ///
    /// returns: Result<Vec<OperationLog>, Error> (all uncompleted operations)
    pub fn get_un_completed_operations(
        &self,
        op_logger: &mut OperationLogger,
    ) -> Vec<OperationLog> {
        op_logger
            .get_all_logs_from_start(&self.last_op_start)
            .into_iter()
            .filter(|op| {
                if op.try_counter >= MAX_TRY_COUNTER && !op.completed {
                    op_logger
                        .write_to_fail_log(&op.op_type)
                        .expect("cant write op to fail log");
                    op_logger
                        .mark_log_as_completed(&op.start_location)
                        .expect("cant mark op as completed");
                    false
                } else {
                    !op.completed
                }
            })
            .collect()
    }
    pub fn restore(&self) -> Result<(), LoggerError> {
        let restorer_path_as_string = Self::get_restorer_path(&self.name);
        let nodes_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, self.name, VALUES_FILE_ENDING, FILE_ENDING
        );
        let values_path = format!(
            "{}\\{}{}{}",
            restorer_path_as_string, self.name, NODES_FILE_ENDING, FILE_ENDING
        );
        let config_path = format!(
            "{}\\{}{}",
            restorer_path_as_string, self.name, RESTORER_SETTINGS_FILE_ENDING
        );

        copy_file(
            &format!("{}{}{}", self.name, VALUES_FILE_ENDING, FILE_ENDING),
            &nodes_path,
        )
        .map_err(|_| LoggerError::CantRestoreFilesFromBackup)?;
        copy_file(
            &format!("{}{}{}", self.name, NODES_FILE_ENDING, FILE_ENDING),
            &values_path,
        )
        .map_err(|_| LoggerError::CantRestoreFilesFromBackup)?;
        Ok(())
    }
}
pub struct Logger {
    op_logger: OperationLogger,
    gen_logger: GeneralLogger<File>,
    restorer: Restorer,
    name: String,
    general_logger_filename: Option<String>,
}
impl Logger {
    pub fn log_operation(&mut self, op: OperationType) -> Result<OperationLog, LoggerError> {
        let op = self.op_logger.log_operation(op)?;
        if self.restorer.should_update(op.id) {
            self.restorer.update(&self.op_logger)?;
            self.save_changes_to_config_file()?;
        }
        Ok(op)
    }
    pub fn mark_operation_as_completed(
        &mut self,
        operation: &OperationLog,
    ) -> Result<(), LoggerError> {
        self.op_logger
            .mark_log_as_completed(&operation.start_location)
    }
    pub fn log_select_operation(&mut self, key: &String) -> Result<OperationLog, LoggerError> {
        self.log_operation(OperationType::Select(key.clone()))
    }
    pub fn log_delete_operation(&mut self, key: &String) -> Result<OperationLog, LoggerError> {
        self.log_operation(OperationType::Delete(key.clone()))
    }
    pub fn log_insert_operation(
        &mut self,
        key: &String,
        value: &Vec<u8>,
    ) -> Result<OperationLog, LoggerError> {
        self.log_operation(OperationType::Insert(key.clone(), value.clone()))
    }
    pub fn log_update_operation(
        &mut self,
        key: &String,
        value: &Vec<u8>,
    ) -> Result<OperationLog, LoggerError> {
        self.log_operation(OperationType::Update(key.clone(), value.clone()))
    }

    pub fn log(&mut self, log_type: LogType, message: String) -> Result<(), LoggerError> {
        self.gen_logger.log(log_type, message)
    }
    pub fn log_info(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Info, message)
    }
    pub fn log_warning(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Warning, message)
    }
    pub fn log_error(&mut self, message: String) -> Result<(), LoggerError> {
        self.log(LogType::Error, message)
    }

    pub fn restore(&mut self, tree: &mut BPlusTree) {
        self.restorer
            .restore()
            .expect("cant restore files form backup files");
        for op in self
            .restorer
            .get_un_completed_operations(&mut self.op_logger)
            .into_iter()
        {
            self.op_logger
                .increment_log_try_counter(&op)
                .expect(&format!(
                    "cant update try counter for operation with id:{}",
                    op.id
                ));
            match op.op_type {
                OperationType::Insert(key, value) => {
                    tree.insert(key, &value)
                        .expect(&format!("cant redo operation with id:{}", op.id));
                }
                OperationType::Select(key) => {
                    tree.search(key)
                        .expect(&format!("cant redo operation with id:{}", op.id));
                }
                OperationType::Delete(key) => {
                    tree.delete(key)
                        .expect(&format!("cant redo operation with id:{}", op.id));
                }
                OperationType::Update(key, new_value) => {
                    tree.update(key, &new_value)
                        .expect(&format!("cant redo operation with id:{}", op.id));
                }
            }
        }
    }
    //[OP_ID - 8b][OP_NAME_LEN - 8b][OP_NAME - OP_NAME_LENb][GEN_ID - 8b][GEN_NAME_LEN - 8b][GEN_NAME - GEN_NAME_LENb]
    fn write_logger_information(
        id: usize,
        file_name: &String,
        file: &mut File,
    ) -> Result<(), LoggerError> {
        file.write_all(&id.to_be_bytes())
            .map_err(|_| LoggerError::CantWriteToConfigFile)?;
        file.write_all(&file_name.len().to_be_bytes())
            .map_err(|_| LoggerError::CantWriteToConfigFile)?;
        file.write_all(file_name.as_bytes())
            .map_err(|_| LoggerError::CantWriteToConfigFile)?;
        Ok(())
    }
    fn read_logger_information(file: &mut File) -> Result<(usize, String), LoggerError> {
        let mut usize_as_byes = [0; SIZE_OF_USIZE];

        file.read_exact(&mut usize_as_byes)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;
        let id = usize::from_be_bytes(usize_as_byes);

        file.read_exact(&mut usize_as_byes)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;

        let mut name_as_bytes = vec![0u8; usize::from_be_bytes(usize_as_byes)];
        file.read_exact(&mut name_as_bytes)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;

        Ok((
            id,
            String::from_utf8(name_as_bytes).map_err(|_| LoggerError::CantReadFromConfigFile)?,
        ))
    }
    fn save_changes_to_config_file(&self) -> Result<(), LoggerError> {
        let path_to_config_file_as_string = format!("{}.{}", self.name, LOGGER_CONFIG_FILENAME);
        let path_to_config_file = Path::new(&path_to_config_file_as_string);
        let mut open_options = OpenOptions::new();

        let mut file = open_options
            .write(true)
            .open(path_to_config_file)
            .map_err(|_| LoggerError::CantWriteToConfigFile)?;

        file.seek(io::SeekFrom::Start(0))
            .map_err(|_| LoggerError::CantWriteToConfigFile)?;
        Self::write_logger_information(
            self.op_logger.id,
            &format!("{}{}", self.name, OPERATION_LOGGER_FILE_ENDING),
            &mut file,
        )?;
        let fname = "".to_string();
        Self::write_logger_information(
            self.gen_logger.id,
            if let Some(fname) = &self.general_logger_filename {
                fname
            } else {
                &fname
            },
            &mut file,
        )
    }
    fn load_from_logger_config_file(
        name: &String,
    ) -> Result<(OperationLogger, GeneralLogger<File>, Option<String>), LoggerError> {
        let path_to_config = format!("{}.{}", name, LOGGER_CONFIG_FILENAME);
        let path_to_config_file = Path::new(&path_to_config);
        let mut open_options = OpenOptions::new();

        let mut file = open_options
            .read(true)
            .write(true)
            .open(path_to_config_file)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;
        file.seek(io::SeekFrom::Start(0))
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;
        let (op_id, op_path) = Self::read_logger_information(&mut file)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;
        let (gen_id, gen_path) = Self::read_logger_information(&mut file)
            .map_err(|_| LoggerError::CantReadFromConfigFile)?;

        Ok((
            OperationLogger {
                log_file: RwLock::new(
                    open_options
                        .open(&op_path)
                        .map_err(|_| LoggerError::CantReadFromConfigFile)?,
                ),
                id: op_id,
            },
            if gen_path == "" {
                GeneralLogger::default()
            } else {
                GeneralLogger::new(
                    open_options
                        .open(&gen_path)
                        .map_err(|_| LoggerError::CantReadFromConfigFile)?,
                    gen_id,
                )
            },
            if gen_path == "" { Some(gen_path) } else { None },
        ))
    }
    pub fn new(name: &String) -> Result<Self, LoggerError> {
        let path_to_config_file_as_string = format!("{}.{}", name, LOGGER_CONFIG_FILENAME);
        let path_to_config_file = Path::new(&path_to_config_file_as_string);
        if path_to_config_file.exists() {
            let (op_logger, gen_logger, general_logger_filename) =
                Self::load_from_logger_config_file(name)?;
            Ok(Self {
                op_logger,
                gen_logger,
                restorer: Restorer::load(name.clone())?,
                name: name.clone(),
                general_logger_filename,
            })
        } else {
            let mut open_options = OpenOptions::new();
            let config_file = open_options
                .create(true)
                .write(true)
                .read(true)
                .open(path_to_config_file)
                .map_err(|_| LoggerError::CantWriteToConfigFile)?;
            let ret = Self {
                op_logger: OperationLogger {
                    log_file: RwLock::new(
                        open_options
                            .open(&format!("{}{}", name, OPERATION_LOGGER_FILE_ENDING))
                            .map_err(|_| LoggerError::CantWriteToConfigFile)?,
                    ),
                    id: 1,
                },
                gen_logger: GeneralLogger::new(
                    open_options
                        .open(&format!("{}{}", name, ".log"))
                        .map_err(|_| LoggerError::CantWriteToConfigFile)?,
                    1,
                ),
                restorer: Restorer::load(name.clone())?,
                name: name.clone(),
                general_logger_filename: Some(format!("{}{}", name, ".log")),
            };
            ret.save_changes_to_config_file()?;

            Ok(ret)
        }
    }
    pub fn get_last_inserted_key(&mut self) -> Option<String> {
        self.op_logger.get_all_logs_from_start(&self.restorer.last_op_start).into_iter().rev()
            .find_map(|operation_log: OperationLog| match operation_log.op_type {
                OperationType::Insert(key, _) => Some(key),
                _ => None,
            })
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
            id: 1,
        };

        // Act
        let log = logger
            .log_operation(OperationType::Insert(
                "key".to_string(),
                "value".to_string().as_bytes().to_vec(),
            ))
            .unwrap();

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
            id: 1,
        };
        let log = logger
            .log_operation(OperationType::Insert(
                "key".to_string(),
                "value".to_string().as_bytes().to_vec(),
            ))
            .unwrap();

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
            id: 1,
        };
        let log = logger
            .log_operation(OperationType::Insert(
                "key".to_string(),
                "value".to_string().as_bytes().to_vec(),
            ))
            .unwrap();

        // Act
        logger.mark_log_as_completed(&log.start_location).unwrap();
        let mut file = logger.log_file.write().unwrap();
        let mut completed_byte = [0u8];
        file.seek(io::SeekFrom::Start(
            log.start_location as u64 + ID_SIZE as u64,
        ))
        .unwrap();
        file.read_exact(&mut completed_byte).unwrap();

        // Assert
        assert_eq!(completed_byte[0], 0x01); // Should be marked as completed
    }

    #[test]
    fn test_default_general_logger() {
        let logger: GeneralLogger<File> = Default::default();
        assert_eq!(logger.id, 1);
        assert!(logger.output.is_none());
    }

    #[test]
    fn test_new_general_logger_with_file() {
        let temp_file = File::create("test_log_file.log").expect("Failed to create temp file");
        let logger = GeneralLogger::new(temp_file.try_clone().unwrap(), 1);

        assert_eq!(logger.id, 1);
        assert!(logger.output.is_some());
    }

    #[test]
    fn test_log_info_to_stdout() {
        let mut logger = GeneralLogger::new(std::io::stdout().lock(), 1);

        let result = logger.info("Test message".to_string());

        assert!(result.is_ok());
        // Check if something was written to stdout (cannot really capture stdout in a test)
    }

    #[test]
    fn test_log_error_to_file() {
        let temp_file = File::create("test_log_file.log").expect("Failed to create temp file");
        let mut logger = GeneralLogger::new(temp_file.try_clone().unwrap(), 1);

        let result = logger.error("Test error message".to_string());

        assert!(result.is_ok());

        let mut contents = String::new();
        let mut file = File::open("test_log_file.log").expect("Failed to open temp file");
        file.read_to_string(&mut contents)
            .expect("Failed to read temp file contents");

        assert!(contents.contains("Error"));
        assert!(contents.contains("Test error message"));

        // Clean up: Remove the created file
        fs::remove_file("test_log_file.log").expect("Failed to remove temp file");
    }

    #[test]
    fn test_log_warning_to_file() {
        let temp_file = File::create("test_log_file.log").expect("Failed to create temp file");
        let mut logger = GeneralLogger::new(temp_file.try_clone().unwrap(), 1);

        let result = logger.warning("Test warning message".to_string());

        assert!(result.is_ok());

        let mut contents = String::new();
        let mut file = File::open("test_log_file.log").expect("Failed to open temp file");
        file.read_to_string(&mut contents)
            .expect("Failed to read temp file contents");

        assert!(contents.contains("Warning"));
        assert!(contents.contains("Test warning message"));

        // Clean up: Remove the created file
        fs::remove_file("test_log_file.log").expect("Failed to remove temp file");
    }

    #[test]
    fn test_log_to_file() {
        let temp_file = File::create("test_log_file.log").expect("Failed to create temp file");
        let mut logger = GeneralLogger::new(temp_file.try_clone().unwrap(), 1);

        let result = logger.log(LogType::Info, "Test log message".to_string());

        assert!(result.is_ok());

        let mut contents = String::new();
        let mut file = File::open("test_log_file.log").expect("Failed to open temp file");
        file.read_to_string(&mut contents)
            .expect("Failed to read temp file contents");

        assert!(contents.contains("Info"));
        assert!(contents.contains("Test log message"));

        // Clean up: Remove the created file
        fs::remove_file("test_log_file.log").expect("Failed to remove temp file");
    }

    const TEST_LOG_FILE_PATH: &str = "test_operation_log_file.log";

    #[test]
    fn test_new_operation_logger() {
        let log_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let logger = OperationLogger::new(log_file, 1);

        assert_eq!(logger.id, 1);
    }

    #[test]
    fn test_log_operation() {
        let log_file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let result = logger.log_operation(op_type.clone());

        assert!(result.is_ok());
        let log = result.unwrap();

        assert_eq!(log.id, 1);
        assert_eq!(log.completed, false);

        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_read_log() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let log = logger
            .log_operation(OperationType::Insert(
                "key".to_string(),
                "value".to_string().as_bytes().to_vec(),
            ))
            .expect("Failed to log operation");
        eprintln!("log.start_location = {:#?}", log.start_location);
        let read_log = logger
            .read_log(&log.start_location)
            .expect("Failed to read log");

        assert_eq!(read_log.id, log.id);
        assert_eq!(read_log.completed, log.completed);

        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_set_log_as_completed() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let log = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let result = logger.mark_log_as_completed(&log.start_location);

        assert!(result.is_ok());
        let log_after = logger
            .read_log(&log.start_location)
            .expect("cant read log from file");
        assert!(log_after.completed);
        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_increment_log_try_counter() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let log = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let result = logger.increment_log_try_counter(&log);

        assert!(result.is_ok());
        let log_after = logger
            .read_log(&log.start_location)
            .expect("cant read log from file");
        assert_eq!(log_after.try_counter, log.try_counter + 1);
        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_get_last_completed_operation() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let log1 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");
        logger
            .mark_log_as_completed(&log1.start_location)
            .expect("cnat mark log 1 as completed");
        let op_type = OperationType::Update(
            "key".to_string(),
            "new_value".to_string().as_bytes().to_vec(),
        );
        let _log2 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let result = logger.get_last_completed_operation(&0);

        assert!(result.is_ok());
        let log = result.unwrap();

        assert_eq!(
            log.op_type,
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec())
        );

        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_get_all_logs_from_start() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let _log1 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let op_type = OperationType::Update(
            "key".to_string(),
            "new_value".to_string().as_bytes().to_vec(),
        );
        let log2 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");
        println!("log2 start={}", log2.start_location);
        let logs = logger.get_all_logs_from_start(&0);

        assert_eq!(logs.len(), 2);

        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    #[test]
    fn test_get_all_logs() {
        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_LOG_FILE_PATH)
            .expect("Failed to create test log file");
        let mut logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());
        let _log1 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let op_type = OperationType::Update(
            "key".to_string(),
            "new_value".to_string().as_bytes().to_vec(),
        );
        let _log2 = logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        let logs = logger.get_all_logs();

        assert_eq!(logs.len(), 2);

        // Clean up: Remove the created file
        fs::remove_file(TEST_LOG_FILE_PATH).expect("Failed to remove test log file");
    }

    const TEST_RESTORER_DIR: &str = "test_restorer";
    const TEST_OP_LOGGER_FILE_PATH: &str = "test_op_logger.log";

    #[test]
    fn test_load_restorer() {
        let restorer_name = "test_load_restorer";
        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let restorer = Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");

        assert_eq!(restorer.name, restorer_name);
        assert_eq!(restorer.last_op_id, 0);
        assert_eq!(restorer.last_op_start, 0);

        // Clean up: Remove the created directory
        fs::remove_dir_all(&Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
    }

    #[test]
    fn test_update_restorer() {
        let restorer_name = "test_update_restorer";
        let restorer_dir = format!("{}\\{}", TEST_RESTORER_DIR, restorer_name);

        // Create a test directory with dummy files

        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let mut restorer =
            Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");

        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_OP_LOGGER_FILE_PATH)
            .expect("Failed to create test log file");
        let mut op_logger = OperationLogger::new(log_file, 1);

        let op_type =
            OperationType::Insert("key".to_string(), "value".to_string().as_bytes().to_vec());

        let log = op_logger
            .log_operation(op_type.clone())
            .expect("Failed to log operation");

        op_logger
            .mark_log_as_completed(&log.start_location)
            .expect("cant mark log as complete");

        restorer
            .update(&op_logger)
            .expect("Failed to update restorer");

        assert_eq!(restorer.last_op_id, 1);

        // Clean up: Remove the created files and directory
        fs::remove_file(TEST_OP_LOGGER_FILE_PATH).expect("Failed to remove test log file");
        fs::remove_dir_all(Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }

    #[test]
    fn test_should_update_restorer() {
        let restorer_name = "test_should_update_restorer";
        let restorer_dir = format!("{}\\{}", TEST_RESTORER_DIR, restorer_name);

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let restorer = Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");

        let should_update = restorer.should_update(50);

        assert!(should_update);

        // Clean up: Remove the created directory
        fs::remove_dir_all(Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }

    #[test]
    fn test_get_last_backed_operation_id() {
        let restorer_name = "test_get_last_backed_operation_id";
        let restorer_dir = format!("{}\\{}", TEST_RESTORER_DIR, restorer_name);

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let mut restorer =
            Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");
        let last_backed_id = restorer.get_last_backed_operation_id();

        assert_eq!(last_backed_id, 0);

        // Clean up: Remove the created directory
        fs::remove_dir_all(Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }

    #[test]
    fn test_get_last_backed_operation_start() {
        let restorer_name = "test_get_last_backed_operation_start";
        let restorer_dir = format!("{}\\{}", TEST_RESTORER_DIR, restorer_name);

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let restorer = Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");

        let last_backed_start = restorer.get_last_backed_operation_start();

        assert_eq!(last_backed_start, 0);

        // Clean up: Remove the created directory
        fs::remove_dir_all(Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }

    #[test]
    fn test_get_problematic_operations() {
        let restorer_name = "test_get_problematic_operations";

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", restorer_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", restorer_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");

        let restorer = Restorer::load(restorer_name.to_string()).expect("Failed to load restorer");

        let log_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(TEST_OP_LOGGER_FILE_PATH)
            .expect("Failed to create test log file");
        let mut op_logger = OperationLogger::new(log_file, 1);

        let log = op_logger
            .log_operation(OperationType::Insert(
                "hello".to_string(),
                "world".to_string().as_bytes().to_vec(),
            ))
            .expect("cant log op");
        op_logger
            .mark_log_as_completed(&log.start_location)
            .expect("cant mark first op as complete");
        let _log = op_logger
            .log_operation(OperationType::Insert(
                "bye".to_string(),
                "yosef".to_string().as_bytes().to_vec(),
            ))
            .expect("cant log op");
        let log = op_logger
            .log_operation(OperationType::Insert(
                "bongo".to_string(),
                "mongo".to_string().as_bytes().to_vec(),
            ))
            .expect("cant log op");

        (0..=MAX_TRY_COUNTER + 1).for_each(|_| {
            let log = op_logger
                .read_log(&log.start_location)
                .expect("cant re read log");
            op_logger
                .increment_log_try_counter(&log)
                .expect("cant inc op try counter");
        });
        let log = op_logger
            .read_log(&log.start_location)
            .expect("cant re read log");

        let problematic_ops = restorer.get_un_completed_operations(&mut op_logger);
        assert_eq!(problematic_ops.len(), 1);

        // Clean up: Remove the created files and directory
        fs::remove_file(TEST_OP_LOGGER_FILE_PATH).expect("Failed to remove test log file");
        fs::remove_dir_all(Restorer::get_restorer_path(&restorer.name))
            .expect("Failed to remove test restorer directory");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            restorer_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }
    #[test]
    fn test_logger_new() {
        let logger_name = "test_logger_new".to_string();

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", logger_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", logger_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
        .expect("Failed to create test nodes file");
        let _logger = Logger::new(&logger_name).expect("cant create new logger");
        fs::remove_file(&format!(
            "{}{}{}",
            logger_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            logger_name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .expect("cant remove nodes");
    }
    #[test]
    fn test_get_last_inserted_key() {
        let logger_name = "test_logger_new".to_string();

        // Create a test directory with dummy files
        fs::write(
            &format!("{}{}{}", logger_name, VALUES_FILE_ENDING, FILE_ENDING),
            b"",
        )
            .expect("Failed to create test values file");
        fs::write(
            &format!("{}{}{}", logger_name, NODES_FILE_ENDING, FILE_ENDING),
            b"",
        )
            .expect("Failed to create test nodes file");
        let mut logger = Logger::new(&logger_name).expect("cant create new logger");
        
        let key = "le key".to_string();
        let value = "le value".to_string().as_bytes().to_vec();
        logger.log_insert_operation(&key, &value).unwrap();
        assert_eq!(logger.get_last_inserted_key(), Some(key));


        fs::remove_file(&format!(
            "{}{}{}",
            logger_name, VALUES_FILE_ENDING, FILE_ENDING
        ))
            .expect("cant remove values");
        fs::remove_file(&format!(
            "{}{}{}",
            logger_name, NODES_FILE_ENDING, FILE_ENDING
        ))
            .expect("cant remove nodes");
    }
}
