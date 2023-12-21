use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    sync::{Arc, Mutex, RwLock, RwLockReadGuard},
};
const BLOOM_FILTER_PATH: &str = "bloom_filter.dat";
const MIN_INSERTS_TO_WRITE_BLOOM_FILTER_ON_DISK: usize = 50;
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
pub enum KeyValueError {
    LoggerError(LoggerError),
    BtreeError(Error),
    FileError(io::Error),
    KeyToLarge,
    CorruptedBloomFilter,
}

type KvResult<T> = Result<T, KeyValueError>;
type ThreadGuard<T> = Arc<Mutex<T>>;
type ThreadProtector<T> = Arc<RwLock<T>>;
pub struct KeyValueStore {
    name: String,
    logger: ThreadGuard<Logger>,
    overwatch: ThreadGuard<Overwatch<String>>,
    tree: ThreadProtector<BPlusTree>,
    bloom_filter: ThreadProtector<BloomFilter>,
    insert_count: ThreadGuard<usize>,
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

impl KeyValueStore {
    fn recover(&mut self) {
        let mut tree = self.tree.write().unwrap();
        self.logger.lock().unwrap().restore(&mut tree);
    }
    /// # Description
    /// function tries to read a bloom filter from disk.
    /// # Arguments
    ///
    /// * `name`: name of the key value store
    ///
    /// returns: Result<BloomFilter, KeyValueError>

    fn load_bloom_filter_from_file(name: &String) -> KvResult<BloomFilter> {
        let mut bloom_filter_data = vec![0u8; M as usize];

        let mut options = OpenOptions::new();
        let mut file = options
            .read(true)
            .open(format!("{}.{}", name, BLOOM_FILTER_PATH))?;
        file.read_exact(&mut bloom_filter_data)?;
        println!(
            "read bloom filter array all data is false? {}",
            bloom_filter_data.iter().all(|&x| x == 0x00).to_string()
        );
        let mut bit_array = vec![false; M as usize];

        for (&byte, bit) in bloom_filter_data.iter().zip(bit_array.iter_mut()) {
            *bit = match byte {
                0x00 => Ok(false),
                0x01 => Ok(true),
                _ => Err(KeyValueError::CorruptedBloomFilter),
            }?;
        }

        Ok(BloomFilter { bit_array })
    }
    /// #  Description
    /// function to read kv from disk.
    /// # Arguments
    ///
    /// * `name`: name of the key value store
    ///
    /// returns: Result<KeyValueStore, KeyValueError>
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
                    |e| -> Result<BloomFilter, KeyValueError> {
                        println!("had an error loading the bloom filter {:?}", e);
                        Ok(BloomFilter::new())
                    },
                )?,
            )),
            insert_count: Arc::new(Mutex::new(0)),
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
            Self::save_bloom_filter_on_file(ret.bloom_filter.clone(), &ret.name)
                .expect("cant create kv ");
            ret
        } else {
            panic!("cant create kv ")
        }
    }
    /// # Description
    /// function saves bloom filter on disk.
    /// # Arguments
    ///
    /// * `bloom_filter`: thread protector of bloom filter
    /// * `name`: name of the collection
    ///
    /// returns: Result<(), KeyValueError>

    fn save_bloom_filter_on_file(
        bloom_filter: ThreadProtector<BloomFilter>,
        name: &String,
    ) -> Result<(), KeyValueError> {
        let bloom_filter_data_as_vec_u8;

        //critical section
        {
            let bloom_filter = bloom_filter.read().unwrap();
            bloom_filter_data_as_vec_u8 = bloom_filter
                .get_array()
                .iter()
                .map(|&b| if b { 0x1 } else { 0x0 })
                .collect::<Vec<u8>>();
        }

        fs::write(
            format!("{}.{}", name, BLOOM_FILTER_PATH),
            &bloom_filter_data_as_vec_u8,
        )
        .map_err(|e| KeyValueError::FileError(e))
    }

    /// # Description
    /// function tries to insert value into the key value store
    /// # Arguments
    ///
    /// * `tree`: b+ tree of the key value store
    /// * `logger`: logger of the key value store
    /// * `bloom_filter`: bloom filter of the key value store
    /// * `insert_count`: counter of how many inserts there were (used to optimize bloom filter updates)
    /// * `key`: key to insert
    /// * `value`: value to insert
    ///
    /// returns: Result<usize, KeyValueError>

    fn insert_internal(
        tree: &Arc<RwLock<BPlusTree>>,
        logger: &ThreadGuard<Logger>,
        bloom_filter: &Arc<RwLock<BloomFilter>>,
        insert_count: &ThreadGuard<usize>,
        key: &String,
        value: String,
    ) -> KvResult<usize> {
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

        let ret = {
            let mut tree = tree.write().unwrap();
            tree.insert(key.clone(), &value)?
        };

        {
            let mut bloom_filter = bloom_filter.write().unwrap();
            bloom_filter.insert(&key);
            *(insert_count.lock().unwrap()) += 1;
        }

        {
            let mut logger = logger.lock().unwrap();
            logger.mark_operation_as_completed(&op_log)?;
        }
        Ok(ret)
    }

    /// #  Description
    /// function tries to search for a key in the store
    /// # Arguments
    ///
    /// * `tree`: b+ tree of the store
    /// * `bloom_filter`: bloom filter of the store
    /// * `logger`: logger of the store
    /// * `key`: key to search for
    ///
    /// returns: Result<Option<String>, KeyValueError>
    fn search_internal(
        tree: &ThreadProtector<BPlusTree>,
        bloom_filter: &ThreadProtector<BloomFilter>,
        logger: &ThreadGuard<Logger>,
        key: &String,
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

        Ok(if contains {
            let op_log = {
                let mut logger = logger.lock().unwrap();

                logger.log_select_operation(key)?
            };
            let search = {
                let tree = tree.read().unwrap();
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
                let mut logger = logger.lock().unwrap();

                logger.mark_operation_as_completed(&op_log)?;
            }

            ret
        } else {
            None
        })
    }
    /// #  Description
    ///  function tries to search a range  of keys in the store
    /// # Arguments
    ///
    /// * `tree`: b+ tree of the store
    /// * `logger`: logger of the store
    /// * `start`: start of range
    /// * `end`: end of range
    ///
    /// returns: Result<Vec<String, Global>, KeyValueError>
    ///
    fn range_scan_internal(
        tree: &ThreadProtector<BPlusTree>,
        logger: &ThreadGuard<Logger>,
        start: &String,
        end: &String,
    ) -> KvResult<Vec<String>> {
        if start.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();
                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    end,
                    end.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }
        if end.len() > MAX_KEY_SIZE {
            {
                let mut logger = logger.lock().unwrap();
                logger.log_error(format!(
                    "key:{} has a len of {} and it needs to be less then {}",
                    end,
                    end.len(),
                    MAX_KEY_SIZE
                ))?;
            }
            return Err(KeyValueError::KeyToLarge);
        }

        Ok({
            let search = {
                let tree = tree.read().unwrap();
                tree.range_search(start.clone(), end.clone())?
            };
            let ret = {
                let mut ret = vec![String::default(); search.len()];
                for (key, value_location) in search {
                    let pager = {
                        let tree = tree.read().unwrap();
                        tree.pager.read_value(value_location)
                    };
                    if let Ok(value) = pager {
                        ret.push(
                            EncryptionService::get_instance()
                                .read()
                                .unwrap()
                                .decrypt(value, &key),
                        );
                    }
                }
                ret
            };

            ret
        })
    }
    /// #  Description
    ///  function tries to update a key from the store and calls the associated overwatch function if it has any
    /// # Arguments
    ///
    /// * `tree`: b+ tree of the store
    /// * `bloom_filter`: bloom filter of the store
    /// * `logger`: logger of the store
    /// * `overwatch`: overwatch of the store
    /// * `key`: key to update
    /// * `new_value`: new value to update with
    ///
    /// returns: Result<Option<String>, KeyValueError>
    ///
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
    /// #  Description
    ///  function tries to delete a key from the store and calls the associated overwatch function if it has any
    /// # Arguments
    ///
    /// * `tree`: b+ tree of the store
    /// * `bloom_filter`: bloom filter of the store
    /// * `logger`: logger of the store
    /// * `overwatch`: overwatch of the store
    /// * `key`: key to delete
    ///
    /// returns: Result<Option<String>, KeyValueError>
    ///
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

    /// # Description
    /// function tries to insert a key into the store saves bloom filter on disk all executed by the threadpool
    /// # Arguments
    ///
    /// * `key`: new key to insert
    /// * `value`: new value to insert
    ///
    /// returns: ComputedValue<Option<usize>> (promise of the new value location)

    pub fn insert(&mut self, key: String, value: String) -> ComputedValue<Option<usize>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let name = self.name.clone();
        let insert_count = Arc::clone(&self.insert_count);

        let ret = ThreadPool::get_instance().compute(
            move |_| {
                return match Self::insert_internal(
                    &tree,
                    &logger,
                    &bloom_filter,
                    &insert_count,
                    &key,
                    value,
                ) {
                    Err(e) => {
                        println!(" had an error");
                        logger
                            .lock()
                            .unwrap()
                            .log_error(format!("cant insert to {} because {:?}", name, e))
                            .expect("cant log error in insert");
                        None
                    }
                    Ok(ret) => {
                        logger
                            .lock()
                            .unwrap()
                            .log_info(format!("inserted {} into {}", name, key))
                            .expect("cant log error in insert");
                        Some(ret)
                    }
                };
            },
            (),
        );

        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let name = self.name.clone();
        let insert_count = Arc::clone(&self.insert_count);

        ThreadPool::get_instance().execute(move || {
            let write_to_file = {
                let mut lock = insert_count.lock().unwrap();
                let ret = *lock >= MIN_INSERTS_TO_WRITE_BLOOM_FILTER_ON_DISK;
                *lock = 0;
                ret
            };
            if write_to_file {
                if let Err(e) = Self::save_bloom_filter_on_file(bloom_filter, &name) {
                    println!("had an error while trying to update bloom filter");
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!("cant insert to {} because {:?}", name, e))
                        .expect("cant log error in insert");
                }
            }
        });
        ret
    }
    /// #  Description
    /// function tries to update a key from the store and calls its corresponding overwatch function if it has one all executed by the threadpool
    /// # Arguments
    ///
    /// * `key`: key to update
    /// * `new_value`: new value for key
    ///
    /// returns: ComputedValue<Option<String>> (promise to the new old value of the key if it had any)
    ///

    pub fn update(&mut self, key: String, new_value: String) -> ComputedValue<Option<String>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let overwatch = Arc::clone(&self.overwatch);
        let name = self.name.clone();
        ThreadPool::get_instance().compute(
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
    /// # Description
    /// function searches for a key in the store and returns a promise to the result executed on the threadpool
    /// # Arguments
    ///
    /// * `key`: key to search for
    ///
    /// returns: ComputedValue<Option<String>>

    pub fn search(&self, key: String) -> ComputedValue<Option<String>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let name = self.name.clone();
        let (send, ret) = std::sync::mpsc::channel();

        ThreadPool::get_instance().execute(move || {
            send.send(
                match Self::search_internal(&tree, &bloom_filter, &logger, &key) {
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
            )
            .expect("cant send back result of search");
        });
        ComputedValue::new(ret)
    }
    /// #  Description
    /// function searches for a range of keys in the store and returns a promise to the result executed on the threadpool
    /// # Arguments
    ///
    /// * `start`: start of range
    /// * `end`: end of range
    ///
    /// returns: ComputedValue<Vec<String, Global>> (promise to the value pointer vector)
    pub fn range_scan(&self, start: String, end: String) -> ComputedValue<Vec<String>> {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let name = self.name.clone();

        ThreadPool::get_instance().compute(
            move |_| match Self::range_scan_internal(&tree, &logger, &start, &end) {
                Ok(ret) => {
                    logger
                        .lock()
                        .unwrap()
                        .log_info(format!("got range {}..{} in {}", start, end, name))
                        .expect("cant log error in insert");
                    ret
                }
                Err(e) => {
                    logger
                        .lock()
                        .unwrap()
                        .log_error(format!(
                            "cant find range {}..{} in {} because {:?}",
                            start, end, name, e
                        ))
                        .expect("cant log error in insert");
                    Vec::new()
                }
            },
            (),
        )
    }
    /// #  Description
    /// function tries to delete a key from the store and calls its corresponding overwatch function if it has one all executed by the threadpool
    /// # Arguments
    ///
    /// * `key`: key to delete
    ///
    /// returns: ()

    pub fn delete(&mut self, key: String) {
        let tree = Arc::clone(&self.tree);
        let logger = Arc::clone(&self.logger);
        let bloom_filter = Arc::clone(&self.bloom_filter);
        let overwatch = Arc::clone(&self.overwatch);
        let name = self.name.clone();
        ThreadPool::get_instance().execute(move || {
            if let Err(e) = Self::delete_internal(&tree, &bloom_filter, &logger, &overwatch, &key) {
                logger
                    .lock()
                    .unwrap()
                    .log_error(format!("cant insert to {} because {:?}", name, e))
                    .expect("cant log error in insert");
            };
        });
    }
    /// # Description
    /// function erases the store
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

impl Drop for KeyValueStore {
    fn drop(&mut self) {
        Self::save_bloom_filter_on_file(self.bloom_filter.clone(), &self.name)
            .expect("cant update bloom filter on file");
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
        let insert_counter = Arc::clone(&kv.insert_count);
        for i in 0..10 {
            println!("inserting {}", i);
            KeyValueStore::insert_internal(
                &tree,
                &logger,
                &bloom_filter,
                &insert_counter,
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
            assert_eq!(result, Some(format!("value_{}", i)));
            let result = KeyValueStore::search_internal(
                &tree,
                &bloom_filter,
                &logger,
                &format!("key_{}", i),
            )
            .expect("cant search in kv");
            assert_eq!(result, Some(format!("value_{}", i + 1)));
        }

        println!("finished update");

        // todo merge after b tree bug fixes then add tests for large value counts + delete tests
        kv.erase();
    }
    #[test]
    fn test_kv_crud() {
        let name = "test".to_string();
        let mut kv = KeyValueStore::new(name);
        let mut insert_results = vec![];
        for i in 0..10 {
            insert_results.push(kv.insert(format!("key_{}", i), format!("value_{}", i)));
        }
        insert_results
            .into_iter()
            .for_each(|x| assert!(x.get().is_some()));
        println!("finished inserts");

        let mut search_results = vec![];
        for i in 0..10 {
            println!("started search for {}", i);
            let result = kv.search(format!("key_{}", i));
            println!("queued search for {}", i);
            search_results.push(result);
            println!("add search for {} to handle collection", i);
        }
        println!("finished queueing the searches");
        search_results.into_iter().enumerate().for_each(|(i, res)| {
            println!("checking if index {} is ok", i);
            assert!(res.get() == Some(format!("value_{}", i)));
            println!("index {} is ok", i);
        });
        println!("finished searches");

        let mut search_results = vec![];
        let mut old_values = vec![];
        for i in 0..10 {
            let result = kv.update(format!("key_{}", i), format!("value_{}", i + 1));
            old_values.push(result);
        }
        old_values.into_iter().enumerate().for_each(|(i, res)| {
            assert_eq!(res.get(), Some(format!("value_{}", i)));
        });
        for i in 0..10 {
            let result = kv.search(format!("key_{}", i));
            search_results.push(result);
        }
        search_results.into_iter().enumerate().for_each(|(i, res)| {
            assert_eq!(res.get(), Some(format!("value_{}", i + 1)));
        });
        println!("finished update");

        for i in 0..10 {
            kv.delete(format!("key_{}", i));
        }
        let mut search_results = vec![];

        for i in 0..10 {
            search_results.push(kv.search(format!("key_{}", i)));
        }
        search_results.into_iter().enumerate().for_each(|(i, res)| {
            assert_eq!(res.get(), None);
        });
        kv.erase();
    }
    #[test]
    fn test_kv_insert() {
        let name = "test".to_string();
        let mut kv = KeyValueStore::new(name);
        let mut results = vec![];

        for i in 0..1000 {
            println!("inserting i:{}", i);
            results.push(kv.insert(format!("key_{}", i), format!("value_{}", i)));
        }

        results.into_iter().for_each(|x| assert!(x.get().is_some()));

        println!("finished inserts");
        kv.erase()
    }
}
