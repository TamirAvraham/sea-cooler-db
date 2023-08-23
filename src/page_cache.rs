use std::{
    cell::Cell,
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    sync::{RwLock, RwLockWriteGuard},
};

use crate::{
    error::{map_err, Error, InternalResult},
    pager::PAGE_SIZE,
};
const EMPTY_PAGE: [u8; PAGE_SIZE] = [0; PAGE_SIZE];

struct FileCache {
    file: RwLock<File>,
    page_size: usize,
    cache: RwLock<Vec<u8>>,
    start: Cell<usize>,
    end: Cell<usize>,
    curent_file_page_count: Cell<usize>,
}

impl FileCache {
    pub fn new(page_size: usize, mut file: File) -> Self {
        let curent_file_page_count = (file.metadata().unwrap().len() as usize) / PAGE_SIZE;

        if curent_file_page_count < page_size {
            let new_page_count = page_size - curent_file_page_count;
            for _ in 0..new_page_count {
                file.write_all(&EMPTY_PAGE).unwrap();
            }
        }
        let len = file.metadata().unwrap().len() as usize;
        let curent_file_page_count = (len) / PAGE_SIZE; //remove
        let mut cache = vec![0; PAGE_SIZE * page_size];
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut cache).unwrap();

        Self {
            file: RwLock::new(file),
            page_size,
            cache: RwLock::new(cache),
            start: Cell::new(0),
            end: Cell::new(page_size-1),
            curent_file_page_count: Cell::new(curent_file_page_count),
        }
    }
    #[inline]
    fn write_cache_to_file(
        &self,
        file: &mut RwLockWriteGuard<'_, File>,
        cache: &mut RwLockWriteGuard<'_, Vec<u8>>,
    ) -> InternalResult<()> {
        file.seek(SeekFrom::Start((PAGE_SIZE * self.start.get()) as u64))
            .map_err(map_err(Error::FileError))?;

        file.write_all(&cache).map_err(map_err(Error::FileError))?;
        Ok(())
    }
    pub fn move_cache(&self, start: usize,cache: &mut RwLockWriteGuard<'_, Vec<u8>>) -> InternalResult<()> {
        let mut file = self
            .file
            .write()
            .map_err(map_err(Error::MovingCacheError(start)))?;
        

        self.write_cache_to_file(&mut file, cache)?;

        let new_cache = if self.curent_file_page_count.get() < start + self.page_size {
            let new_page_count = (start + self.page_size) - self.curent_file_page_count.get();
            let mut new_cache = vec![];
            if new_page_count < self.page_size {
                let amount_of_pages_i_already_have =
                    self.curent_file_page_count.get() % self.page_size;

                file.seek(SeekFrom::End(0))
                    .map_err(map_err(Error::MovingCacheError(start)))?;

                new_cache.reserve(amount_of_pages_i_already_have);

                file.read_exact(&mut new_cache)
                    .map_err(map_err(Error::MovingCacheError(start)))?;
            }
            new_cache.extend(vec![0; new_page_count * PAGE_SIZE]);
            self.curent_file_page_count.set(start + self.page_size);

            new_cache
        } else {
            file.seek(SeekFrom::Start(start as u64))
                .map_err(map_err(Error::MovingCacheError(start)))?;
            let mut new_cache = vec![0; self.page_size * PAGE_SIZE];
            file.read_exact(&mut new_cache)
                .map_err(map_err(Error::MovingCacheError(start)))?;
            new_cache
        };

        *(*cache) = new_cache;

        self.start.set(start);
        self.end.set(start + self.page_size);

        Ok(())
    }
}

// i dont think this is a good idea but if we want this this is the correct impl for running write on drop
// impl Drop for FileCache {
//     fn drop(&mut self) {
//         let mut file=self.file.write().unwrap();
//         let mut cache=self.cache.write().unwrap();
//         self.write_cache_to_file(&mut file, &mut cache).unwrap();
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    fn get_file_cache_no_create(page_size: usize) -> FileCache {
        let path = "test_nodes_file.bin";

        let mut options = OpenOptions::new();
        options.read(true).write(true);

        let file = options.open(path).unwrap();
        FileCache::new(page_size, file)
    }
    fn get_file_cache(page_size: usize) -> FileCache {
        let path = "test_nodes_file.bin";

        if fs::metadata(path).is_ok() {
            fs::remove_file(path).unwrap()
        }

        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);

        let file = options.open(path).unwrap();
        FileCache::new(page_size, file)
    }
    #[test]
    fn test_move_cache() {
        let cache = get_file_cache(5);
        let mut data_lock=cache.cache.write().unwrap();
        assert_eq!(cache.curent_file_page_count.get(), 5);

        cache.move_cache(10,&mut data_lock).unwrap();
        assert_eq!(cache.curent_file_page_count.get(), 15);

        cache.move_cache(0,&mut data_lock).unwrap();
        assert_eq!(cache.curent_file_page_count.get(), 15);
    }
    #[test]
    fn test_for_cache_write() {
        let page_size = 5;
        let value = 8usize;

        let cache = get_file_cache(page_size);
        let mut data = cache.cache.write().unwrap();
        let mut file = cache.file.write().unwrap();

        
        data[0..8].copy_from_slice(&value.to_be_bytes());
        cache.write_cache_to_file(&mut file, &mut data).unwrap();

        let cache = get_file_cache_no_create(page_size);
        let read_value =
            usize::from_be_bytes((&cache.cache.read().unwrap()[0..8]).try_into().unwrap());
        
        assert_eq!(value, read_value);

        
        
    }
    #[test]
    fn write_on_move() {
        let page_size = 5;
        let value = 8usize;

        let cache=get_file_cache(page_size);
        
        let mut data = cache.cache.write().unwrap();
        
        let read_start=cache.end.get()*PAGE_SIZE;

        data[read_start..read_start+8].copy_from_slice(&value.to_be_bytes());
        cache.move_cache(page_size*2,&mut data).unwrap();
        

        
        let cache = get_file_cache_no_create(page_size);
        let read_value =
            usize::from_be_bytes((&cache.cache.read().unwrap()[read_start..read_start+8]).try_into().unwrap());
        

        assert_eq!(value, read_value);
    }
}
