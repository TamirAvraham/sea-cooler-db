mod error;
mod pager;
mod node;
mod btree;
mod page_cache;
mod aes128;
mod encryption;
mod thread_pool;
mod bloom_filter;
mod helpers;
mod json;
mod validation_json;
mod logger;
mod overwatch;
mod key_value_store;
mod skip_list;
mod collection;
mod http_parser;
mod database;
mod user_system;
mod http_server;
mod radix_tree;
mod database_api;

use std::io;
use std::io::Write;
use crate::database::DataBase;
use crate::database_api::start_db_api;
use crate::key_value_store::KeyValueStore;

fn get_user_input()->String{
    let mut input_string = String::new();


    io::stdout().flush().unwrap(); // Ensure the print! macro output is displayed immediately

    io::stdin()
        .read_line(&mut input_string)
        .expect("Failed to read line");
    input_string
}
fn help(){
    println!(r#"
            select -> get a value from the kv,
            insert -> insert a key value into the database
            update -> update a key value store
            delete -> delete a key from the store
            range -> get a range of values from the database
            erase -> resets the database
            exit -> exit the program
            "#);
}

const KV_NAME:&str="temp";
fn main() {
    let mut user_input="".to_string();
    let mut kv = KeyValueStore::new(KV_NAME.to_string());

    while user_input!="exit" {
        print!("enter command or help to list commands:\t");
        user_input= get_user_input().trim().parse().unwrap();

        match user_input.as_str() {
            "select" => select(&mut kv),
            "insert" => insert(&mut kv),
            "delete" => delete(&mut kv),
            "update" => update(&mut kv),
            "help" => help(),
            "range" => range(&mut kv),
            "exit" => println!("bye bye"),
            "erase" => {
                kv.erase();
                break;
            },
            "start"=>{
                start_db_api("seacoller".to_string())
            }
            _=> println!("invalid command"),
        }
    }

}


fn range(kv:&mut KeyValueStore){
    print!("enter start:");
    let start=get_user_input();
    print!("enter range end:");
    let end=get_user_input();
    println!("found:");
    kv.range_scan(start,end).get().into_iter().for_each(|s|{
        print!("{} ",s);
    });
    println!("in kv");
}
fn update(kv: &mut KeyValueStore) {
    print!("enter key:");
    let key = get_user_input();
    print!("enter new value:");
    let value=get_user_input();

    let result=kv.update(key,value).get();
    if let Some(value) = result {
        println!("old value was {}",value.0);
    }else {
        println!("key was not found");
    }
}

fn delete(kv: &mut KeyValueStore)  {
    print!("enter key:");
    let key = get_user_input();
    kv.delete(key);

}

fn select(kv: &mut KeyValueStore)  {
    print!("enter key:");
    let key = get_user_input();
    let result=kv.search(key).get();
    if let Some(value) = result {
        println!("value is {}",value);
    }else {
        println!("value was not found");
    }

}
fn insert(kv:&mut KeyValueStore){
    print!("enter key:");
    let key = get_user_input();
    print!("enter value:");
    let value=get_user_input();

    kv.insert(key,value).get();
}
