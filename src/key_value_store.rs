use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};
const BLOOM_FILTER_PATH: &str = "bloom_filter.dat";
use crate::{
    bloom_filter::{self, BloomFilter, M},
    btree::{self, BPlusTree, DEFAULT_T, FILE_ENDING, NODES_FILE_ENDING, VALUES_FILE_ENDING},
    encryption::EncryptionService,
    error::Error,
    logger::{self, LogType, Logger, LoggerError, OPERATION_LOGGER_FILE_ENDING, RESTORER_DIR},
    node::MAX_KEY_SIZE,
    overwatch::{self, Overwatch},
    thread_pool::{ComputedValue, ThreadPool},
};
#[derive(Debug)]
enum KeyValueError {
    LoggerError(LoggerError),
    BtreeError(Error),
    FileError(io::Error),
    KeyToLarge,
    CorruptedBloomFilter,
}

type KvResult<T> = Result<T, KeyValueError>;
type ThreadGuard<T> = Arc<Mutex<T>>;
type ThreadProtector<T> = Arc<RwLock<T>>;
struct KeyValueStore {
    name: String,
    logger: ThreadGuard<Logger>,
    overwatch: ThreadGuard<Overwatch<String>>,
    tree: ThreadProtector<BPlusTree>,
    bloom_filter: ThreadProtector<BloomFilter>,
}
impl From<LoggerError> for KeyValueError {
    fn from(item: LoggerError) -> Self {
        KeyValueError::LoggerError(item)
    }
}
impl From<Error> for KeyValueError {
    fn from(item: Error) -> Self {
        KeyValueError::BtreeError(item)
    }
}
impl From<io::Error> for KeyValueError {
    fn from(item: io::Error) -> Self {
        KeyValueError::FileError(item)
    }
}
/*
   shit i need to impl:
   add graceful crashes
   test restore on logger
   maybe start collections
*/
impl KeyValueStore {
    fn recover(&mut self) {
        let mut tree = self.tree.write().unwrap();
        self.logger.lock().unwrap().restore(&mut tree);
    }
    fn load_bloom_filter_from_file(name: &String) -> KvResult<BloomFilter> {
        let mut bloom_filter_data = vec![0u8; M as usize];

        let mut options = OpenOptions::new();
        let mut file = options
            .read(true)
            .open(format!("{}.{}", name, BLOOM_FILTER_PATH))?;
        file.read_exact(&mut bloom_filter_data)?;

        let mut bit_array = vec![false; M as usize];
        for byte in bloom_filter_data {
            match byte {
                0x0 => bit_array.push(false),
                0x01 => bit_array.push(true),
                _ => return Err(KeyValueError::CorruptedBloomFilter),
            }
        }
        Ok(BloomFilter { bit_array })
    }
    fn load(name: String) -> KvResult<Self> {
        let tree = Arc::new(RwLock::new(
            btree::BTreeBuilder::new()
                .name(&name)
                .t(DEFAULT_T)
                .path(name.clone())
                .build()?,
        ));

        let ret = Ok(Self {
            name: name.clone(),
            logger: Arc::new(Mutex::new(Logger::new(&name)?)),
            tree,
            overwatch: Arc::new(Mutex::new(Overwatch::new())),
            bloom_filter: Arc::new(RwLock::new(
                Self::load_bloom_filter_from_file(&name).or_else(
                    |_| -> Result<BloomFilter, KeyValueError> { Ok(BloomFilter::new()) },
                )?,
            )),
        });

        ret.as_ref()
            .unwrap()
            .logger
            .lock()
            .unwrap()
            .log_info("loaded Key value store from files".to_string())?;

        ret
    }
    pub fn new(name: String) -> Self {
        if let Ok(mut ret) = Self::load(name) {
            ret.recover();
            ret.save_bloom_filter_on_file().expect("cant create kv ");
            ret
        } else {
            panic!("cant create kv ")
        }
    }
    fn save_bloom_filter_on_file(&self) -> Result<(), KeyValueError> {
        let bloom_filter_data_as_vec_u8;

        //critical section
        {
            let bloom_filter = self.bloom_filter.read().unwrap();
            bloom_filter_data_as_vec_u8 = bloom_filter
                .get_array()
                .iter()
                .map(|&b| if b { 0x1 } else { 0x0 })
                .collect::<Vec<u8>>();
        }

        fs::write(
            format!("{}.{}", self.name, BLOOM_FILTER_PATH),
            &bloom_filter_data_as_vec_u8,
        )
        .map_err(|e| KeyValueError::FileError(e))
    }

    fn insert_internal(
        tree: &Arc<RwLock<BPlusTree>>,
        logger: &ThreadGuard<Logger>,
        bloom_filter: &Arc<RwLock<BloomFilter>>,
        key: &String,
        value: String,
    ) -> KvResult<()> {
        if key.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();

                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    key,
                    key.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }

        let value = EncryptionService::get_instance()
            .read()
            .unwrap()
            .encrypt(value, key);

        let op_log = {
            let mut logger = logger.lock().unwrap();
            logger.log_insert_operation(key, &value)?
        };

        {
            let mut tree = tree.write().unwrap();
            tree.insert(key.clone(), &value)?;
        }

        {
            let mut bloom_filter = bloom_filter.write().unwrap();
            bloom_filter.insert(&key);
        }

        {
            let mut logger = logger.lock().unwrap();
            logger.mark_operation_as_completed(&op_log)?;
        }
        Ok(())
    }
    fn search_internal(
        tree: &ThreadProtector<BPlusTree>,
        bloom_filter: &ThreadProtector<BloomFilter>,
        logger: &ThreadGuard<Logger>,
        key: &String,
    ) -> KvResult<Option<String>> {
        println!("started search for {}",key);
        if key.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();
                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    key,
                    key.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }

        let contains = {
            println!("locking bloom filter in {}",key);
            let bloom_filter = bloom_filter.read().unwrap();
            println!("locked bloom filter in {}",key);
            bloom_filter.contains(&key)
        };
        

        Ok(if contains {
            let op_log = {
                println!("locking logger in {}",key);
                let mut logger = logger.lock().unwrap();
                println!("locked logger in {}",key);

                logger.log_select_operation(key)?
            };

            let search = {
                println!("locking tree in {}",key);
                let tree = tree.read().unwrap();
                println!("locked tree in {}",key);
                tree.search(key.clone())
            };
            let ret = if let Some(ret_encrypted) = search? {
                Some(
                    EncryptionService::get_instance()
                        .read()
                        .unwrap()
                        .decrypt(ret_encrypted, key),
                )
            } else {
                None
            };

            {
                println!("locking logger in {}",key);
                let mut logger = logger.lock().unwrap();
                println!("locked logger in {}",key);

                logger.mark_operation_as_completed(&op_log)?;
            }

            ret
        } else {
            None
        })
    }
    fn update_internal(
        logger: &ThreadGuard<Logger>,
        tree: &ThreadProtector<BPlusTree>,
        bloom_filter: &ThreadProtector<BloomFilter>,
        overwatch: &ThreadGuard<Overwatch<String>>,
        key: &String,
        new_value: String,
    ) -> KvResult<Option<String>> {
        if key.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();
                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    key,
                    key.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }
        let contains = {
            let bloom_filter = bloom_filter.read().unwrap();
            bloom_filter.contains(&key)
        };
        if contains {
            let new_value_as_string = new_value.clone();
            let new_value = EncryptionService::get_instance()
                .read()
                .unwrap()
                .encrypt(new_value, key);

            let op_log = {
                let mut logger = logger.lock().unwrap();
                logger.log_update_operation(key, &new_value)?
            };

            let update = {
                let mut tree = tree.write().unwrap();
                tree.update(key.clone(), &new_value)
            };

            let ret = if let Some(ret_encrypted) = update? {
                Some(
                    EncryptionService::get_instance()
                        .read()
                        .unwrap()
                        .decrypt(ret_encrypted, key),
                )
            } else {
                None
            };

            if ret.is_some() {
                let mut overwatch = overwatch.lock().unwrap();
                overwatch.get_update(key, new_value_as_string)
            }

            {
                let mut logger = logger.lock().unwrap();
                logger.mark_operation_as_completed(&op_log)?;
            }
            Ok(ret)
        } else {
            Ok(None)
        }
    }
    fn delete_internal(
        tree: &ThreadProtector<BPlusTree>,
        bloom_filter: &ThreadProtector<BloomFilter>,
        logger: &ThreadGuard<Logger>,
        overwatch: &ThreadGuard<Overwatch<String>>,
        key: &String,
    ) -> KvResult<()> {
        if key.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();

                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    key,
                    key.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }

        let contains = {
            let bloom_filter = bloom_filter.read().unwrap();
            bloom_filter.contains(&key)
        };
        if contains {
            let op_log = {
                let mut logger = logger.lock().unwrap();
                logger.log_delete_operation(key)?
            };

            let vec = {
                let tree = tree.read().unwrap();

                tree.search(key.clone())?
            };
            let ret = if let Some(ret_encrypted) = vec {
                Some(
                    EncryptionService::get_instance()
                        .read()
                        .unwrap()
                        .decrypt(ret_encrypted, key),
                )
            } else {
                None
            };
            {
                let mut tree = tree.write().unwrap();
                tree.delete(key.clone())?;
            }

            {
                let mut overwatch = overwatch.lock().unwrap();

                if let Some(last_value) = ret {
                    overwatch.get_delete(key, last_value);
                    overwatch.remove_delete(key);
                }
                overwatch.remove_update(key);
            }

            {
                let mut logger = logger.lock().unwrap();

                logger.mark_operation_as_completed(&op_log)?;
            }
        }
        Ok(())
    }

    pub fn insert(&mut self, key: String, value: String) {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let name = self.name.clone();

        ThreadPool::get_instance()
            .write()
            .unwrap()
            .execute(move || {
                if let Err(e) = Self::insert_internal(&tree, &logger, &bloom_filter, &key, value) {
                    println!(" had an error");
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!("cant insert to {} because {:?}", name, e))
                        .expect("cant log error in insert");
                } else {
                    logger
                        .lock()
                        .unwrap()
                        .log_info(format!("inserted {} into {}", name, key))
                        .expect("cant log error in insert");
                };
            });
    }
    pub fn update(&mut self, key: String, new_value: String) -> ComputedValue<Option<String>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let overwatch = Arc::clone(&self.overwatch);
        let name = self.name.clone();
        ThreadPool::get_instance().write().unwrap().compute(
            move |_| match Self::update_internal(
                &logger,
                &tree,
                &bloom_filter,
                &overwatch,
                &key,
                new_value,
            ) {
                Ok(ret) => ret,
                Err(e) => {
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!("cant insert to {} because {:?}", name, e))
                        .expect("cant log error in insert");
                    None
                }
            },
            (),
        )
    }
    pub fn search(&self, key: String) -> ComputedValue<Option<String>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let name = self.name.clone();

        ThreadPool::get_instance().write().unwrap().compute(
            move |_| match Self::search_internal(&tree, &bloom_filter, &logger, &key) {
                Ok(ret) => {
                    logger
                        .lock()
                        .unwrap()
                        .log_info(format!("found {} in {}", key, name))
                        .expect("cant log error in insert");
                    ret
                }
                Err(e) => {
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!("cant search {} in {} because {:?}", key, name, e))
                        .expect("cant log error in insert");
                    None
                }
            },
            (),
        )
    }
    pub fn delete(&mut self, key: String) {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let overwatch = Arc::clone(&self.overwatch);
        let name = self.name.clone();
        ThreadPool::get_instance()
            .write()
            .unwrap()
            .execute(move || {
                if let Err(e) =
                    Self::delete_internal(&tree, &bloom_filter, &logger, &overwatch, &key)
                {
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!("cant insert to {} because {:?}", name, e))
                        .expect("cant log error in insert");
                };
            });
    }
    pub fn erase(self) {
        fs::remove_file(&format!("{}{}", self.name, OPERATION_LOGGER_FILE_ENDING)).unwrap(); // op logger
        fs::remove_file(&format!("{}.{}", self.name, BLOOM_FILTER_PATH)).unwrap(); //bloom filter
        fs::remove_file(&format!("{}{}", self.name, FILE_ENDING)).unwrap(); // btree root id
        fs::remove_file(&format!("{}{}", self.name, ".log")).unwrap(); //general logger
        fs::remove_file(&format!("{}.{}", self.name, logger::LOGGER_CONFIG_FILENAME)).unwrap(); // logger config
        fs::remove_dir_all(&format!("{}_{}", self.name, RESTORER_DIR)).unwrap(); //backup
        fs::remove_file(&format!(
            "{}{}{}",
            self.name, VALUES_FILE_ENDING, FILE_ENDING
        ))
        .unwrap(); // values

        fs::remove_file(&format!(
            "{}{}{}",
            self.name, NODES_FILE_ENDING, FILE_ENDING
        ))
        .unwrap(); //  nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_kv_new() {
        let name = "test".to_string();
        let kv = KeyValueStore::new(name);
        kv.erase();
    }
    #[test]
    fn test_kv_non_multi_threaded() {
        let name = "test".to_string();
        let kv = KeyValueStore::new(name);
        let tree = Arc::clone(&kv.tree);
        let logger = Arc::clone(&kv.logger);
        let bloom_filter = Arc::clone(&kv.bloom_filter);
        let overwatch = Arc::clone(&kv.overwatch);

        for i in 0..10 {
            println!("inserting {}", i);
            KeyValueStore::insert_internal(
                &tree,
                &logger,
                &bloom_filter,
                &format!("key_{}", i),
                format!("value_{}", i),
            )
            .expect("cant insert");
        }
        println!("finished inserts");

        for i in 0..10 {
            let result = KeyValueStore::search_internal(
                &tree,
                &bloom_filter,
                &logger,
                &format!("key_{}", i),
            )
            .expect("cant search in kv");
            assert!(result == Some(format!("value_{}", i)));
        }

        println!("finished searches");

        for i in 0..10 {
            let result = KeyValueStore::update_internal(
                &logger,
                &tree,
                &bloom_filter,
                &overwatch,
                &format!("key_{}", i),
                format!("value_{}", i + 1),
            )
            .expect("cnat update");
            assert!(result == Some(format!("value_{}", i)));
            let result = KeyValueStore::search_internal(
                &tree,
                &bloom_filter,
                &logger,
                &format!("key_{}", i),
            )
            .expect("cant search in kv");
            assert!(result == Some(format!("value_{}", i + 1)));
        }

        println!("finished update");

        // todo merge after b tree bug fixes then add tests for large value counts + delete tests
        kv.erase();
    }
    #[test]
    fn test_kv_crud() {
        let name = "test".to_string();
        let mut kv = KeyValueStore::new(name);

        for i in 0..10 {
            kv.insert(format!("key_{}", i), format!("value_{}", i));
        }
        println!("finished inserts");

        let mut search_results = vec![];
        for i in 0..10 {
            println!("started search for {}",i);
            let result = kv.search(format!("key_{}", i));
            println!("queued search for {}",i);
            search_results.push(result);
            println!("add search for {} to handle collection",i);
        }
        println!("finished queueing the searches");
        search_results.into_iter().enumerate().for_each(|(i, res)| {
            println!("checking if index {} is ok",i);
            assert!(res.get() == Some(format!("value_{}", i)));
            println!("index {} is ok",i);
        });
        println!("finished searches");

        let mut search_results = vec![];
        let mut old_values = vec![];
        for i in 0..10 {
            let result = kv.update(format!("key_{}", i), format!("value_{}", i + 1));
            old_values.push(result);
            let result = kv.search(format!("key_{}", i));
            search_results.push(result);
        }
        old_values.into_iter().enumerate().for_each(|(i, res)| {
            assert!(res.get() == Some(format!("value_{}", i)));
        });
        search_results.into_iter().enumerate().for_each(|(i, res)| {
            assert!(res.get() == Some(format!("value_{}", i + 1)));
        });
        println!("finished update");

        // todo merge after b tree bug fixes then add tests for large value counts + delete tests
        kv.erase();
    }
}
