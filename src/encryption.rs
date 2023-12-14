use std::sync::{RwLockReadGuard, Once, RwLock, Arc};

use crate::{aes128, thread_pool::ThreadPool};



fn pad_key(key:&String)->[u8;16] {
    let mut ret=[0u8;16];

    match key.len()>=16 {
        true => ret.copy_from_slice(&key.as_bytes()[0..16]),
        false => ret[0..key.len()].copy_from_slice(key.as_bytes()),
    }

    ret
}

fn pad_text(mut text:String)->String{
    match text.len()%16==0 {
        true => text,
        false => {
            let mut padding=String::new();
            let padding_size = 16 - (text.len() % 16);

            for _ in 0..padding_size {
                padding.push(' ');
            }

            text.push_str(&padding);
            text
        }
    }
}

pub fn encrypt(key:&String,text:String)->Vec<u8>{
    let key=pad_key(key);
    let text=pad_text(text);
    aes128::encrypt_aes128(&key, text.as_bytes())
}



pub fn decrypt(key:&String,encrypted:Vec<u8>)->String{
    String::from_utf8(aes128::decrypt_aes128(&pad_key(key), &encrypted)).unwrap().trim_end().to_string()
}
static mut SINGLETON:Option<RwLock<EncryptionService>>=None;
static INIT:Once=Once::new();

pub struct EncryptionService {
    threadpool_connection:Arc<ThreadPool>,
}
impl EncryptionService {
    fn new() -> EncryptionService {
        EncryptionService{ threadpool_connection: ThreadPool::get_instance() }
    }

    pub fn encrypt(&self,text:String,key:&String)->Vec<u8>{
        encrypt(key,text)
    }
    pub fn decrypt(&self,text:Vec<u8>,key:&String)->String{
        decrypt(key,text)
    }

    pub fn get_instance()-> &'static mut RwLock<Self>{
        INIT.call_once(||
            unsafe {
                SINGLETON=Some(RwLock::new(Self::new()));
            }
        );

        unsafe {
            SINGLETON.as_mut().unwrap()
        }
    }
}
#[cfg(test)]
mod tests{
    use super::{pad_key, encrypt, decrypt};

    #[test]
    fn test_pad_key() {
        
        let key1 = "1111111111111111".to_string();

        let padded_key=pad_key(&key1);

        assert_eq!(key1.as_bytes(),&padded_key);


        let mut key2="1234".to_string();

        let padded_key=pad_key(&key2);

        key2.push_str("\0\0\0\0\0\0\0\0\0\0\0\0");


        assert_eq!(&padded_key,key2.as_bytes())
    }
    #[test]
    fn test_encrypt(){
        let text="aes is a pain in the ass to write".to_string();

        let key="i hope cha cha 20 will be easier".to_string();


        let encrypted_text=encrypt(&key, text.clone());


        let decrypted_text=decrypt(&key, encrypted_text);

        assert_eq!(text,decrypted_text)
    }

    
}