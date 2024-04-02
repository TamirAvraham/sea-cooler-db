mod aes128;
mod bloom_filter;
mod btree;
mod collection;
mod database;
mod database_api;
mod encryption;
mod error;
mod helpers;
mod http_parser;
mod http_server;
mod json;
mod key_value_store;
mod logger;
mod node;
mod overwatch;
mod page_cache;
mod pager;
mod radix_tree;
mod skip_list;
mod thread_pool;
mod user_system;
mod validation_json;

use crate::database::{DataBase, DataBaseError};
use crate::database_api::{bind_api_to_db, start_db_api};
use crate::json::{JsonData, JsonObject, JsonSerializer};
use crate::key_value_store::KeyValueStore;
use crate::user_system::UserType;
use std::io::Write;
use std::{env, io};

fn get_user_input() -> String {
    let mut input_string = String::new();

    io::stdout().flush().unwrap();

    io::stdin()
        .read_line(&mut input_string)
        .expect("Failed to read line");
    input_string
}
fn get_json_from_user(info: bool, tab_count: u8) -> JsonObject {
    let mut ret = JsonObject::new();
    if info {
        println!("start entering your json key value pairs in this format");
        println!("[key]:[value]\n example pair : yoni:123 will translate to \"yoni\": 123");
        println!("enter }} to end or [key]:{{ to start creating another object");
    }
    for _ in 0..tab_count {
        print!("\t");
    }
    print!("{{\n");
    let tab_count = tab_count + 1;
    loop {
        for _ in 0..tab_count {
            print!("\t");
        }
        let user_input = get_user_input();
        if user_input == "}" {
            break;
        } else if user_input.chars().last() == Some('{') {
            let mut split_input = user_input.split(":");
            let key = split_input.next().unwrap();
            ret.insert(key.to_string(), get_json_from_user(false, tab_count).into());
        } else {
            let mut split_input = user_input.split(":");
            let key = split_input.next().unwrap();
            let value = split_input.next().unwrap();
            ret.insert(
                key.to_string(),
                JsonData::infer_from_string(value.to_string()).unwrap(),
            );
        }
    }
    ret
}
fn user_help() {
    println!(
        r#"
            help -> list all commands
            register -> create a new user
            login -> login to the system
    "#
    );
}
fn help() {
    println!(
        r#"
            help -> list all commands
            select -> get a document from the database,
            insert -> insert a key value into the database
            update -> update a key value store
            delete -> delete a document from the database
            select all-> get all of the documents in a collection
            erase -> resets the database
            exit -> exit the program
            start -> start the database REST api
            create collection -> create a new collection
            logout -> logout of the system

            "#
    );
}

fn main() {
    let mut user_input = "".to_string();
    let mut db = DataBase::new(env::args()
        .nth(1)
        .map_or("seacoller".to_string(), |s| s))
        .expect("Failed to create database");
    let mut user_id = 0;
    while user_input != "exit" {
        if user_id != 0 {
            print!("enter command or help to list commands:\t");
            user_input = get_user_input().trim().parse().unwrap();
            match user_input.as_str() {
                "select" => select(&mut db, user_id),
                "insert" => insert(&mut db, user_id),
                "delete" => delete(&mut db, user_id),
                "update" => update(&mut db, user_id),

                "select all" => select_all(&mut db, user_id),
                "exit" => println!("bye bye"),
                "erase" => {
                    println!("erasing database");
                    break;
                }
                "start" => {
                    println!("starting api");
                    break;
                }
                "create collection" => create_basic_collection(&mut db, user_id),
                "help" => help(),
                "logout" => {
                    logout(&mut db, user_id);
                    user_id = 0;
                }
                _ => println!("invalid command"),
            }
        } else {
            println!("no user found login or register");
            print!("login or register:\t");
            user_input = get_user_input().trim().parse().unwrap();
            match user_input.as_str() {
                "login" => user_id = login(&mut db),
                "register" => user_id = register(&mut db),
                "help" => help(),
                _ => println!("invalid command"),
            }
        }
    }
    if user_input == "start" {
        bind_api_to_db(db);
        loop {}
    } else if user_input == "erase" {
        db.erase(user_id).expect("cant erase");
    }
}

fn select_all(db: &mut DataBase, user_id: u128) {
    let collection = select_collection();
    println!("documents in {} are:", collection);
    db.get_all_documents_from_collection(&collection, user_id)
        .expect("cant get from collection")
        .into_iter()
        .for_each(|(name, doc)| {
            print!("\t name: {}\n", name);
            println!("\t value is {}", JsonSerializer::serialize(doc));
        })
}
fn update(db: &mut DataBase, user_id: u128) {
    let collection = select_collection();
    print!("enter document name:");
    let key = get_user_input();
    print!("enter new value:");
    let value = get_json_from_user(true, 0);
    db.update_collection(&collection, key.clone(), value, user_id)
        .unwrap_or_else(|err| println!("cant update {} in {} because",key,collection));;
}

fn delete(db: &mut DataBase, user_id: u128) {
    let collection = select_collection();
    print!("enter document name:");
    let key = get_user_input();
    db.delete_from_collection(&collection, key.clone(), user_id)
        .unwrap_or_else(|err| println!("cant delete {} from {} because",key,collection));
}

fn select(db: &mut DataBase, user_id: u128) {
    let collection = select_collection();
    print!("enter document name:");
    let key = get_user_input();
    let result = db
        .get_from_collection(&collection, key.clone(), user_id)
        .unwrap_or_else(|err|{
            println!("cant select {} from {} because",key,collection);
            None
        });
    if let Some(value) = result {
        println!("value is {}", JsonSerializer::serialize(value));
    } else {
        println!("value was not found");
    }
}

fn insert(db: &mut DataBase, user_id: u128) {
    let collection = select_collection();
    print!("enter document name:");
    let key = get_user_input();
    print!("enter value:");
    let value = get_json_from_user(true, 0);
    db.insert_into_collection(&collection, key.clone(), value, user_id)
        .unwrap_or_else(|err|{
            println!("cant insert {} into {} because", key, collection);

        });
}
fn select_collection() -> String {
    print!("enter collection name:");
    get_user_input()
}

fn create_basic_collection(db: &mut DataBase, user_id: u128) {
    print!("enter collection name:");
    let name = get_user_input();
    db.create_collection(name, None, user_id)
        .expect("Failed to create collection");
}
fn login(db: &mut DataBase) -> u128 {
    print!("enter username:");
    let username = get_user_input();
    print!("enter password:");
    let password = get_user_input();
    db.login(username, password).unwrap_or_else(|err| {
        println!("login failed since {:?}", err);
        0
    })
}
fn register(db: &mut DataBase) -> u128 {
    print!("enter username:");
    let username = get_user_input();
    print!("enter password:");
    let password = get_user_input();
    print!("enter permissions level: ");
    let permissions = match get_user_input().to_lowercase().trim() {
        "admin" => Ok(UserType::Admin),
        "user" => Ok(UserType::User),
        "guest" => Ok(UserType::Guest),
        _ => Err("invalid permissions level"),
    }
    .expect("invalid permissions level");
    db.signup(username.clone(), password, permissions.get_permissions())
        .unwrap_or_else(|err|{
            println!("cant create new user {} because {:?}",username,err);
            0
        })
}
fn logout(db: &mut DataBase, user_id: u128) {
    db.logout(user_id);
}
