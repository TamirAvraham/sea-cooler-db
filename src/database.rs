use crate::collection::{Collection, CollectionError};
use crate::json::{JsonData, JsonDeserializer, JsonError, JsonObject, JsonSerializer};
use crate::key_value_store::{BLOOM_FILTER_PATH, KeyValueStore};
use crate::skip_list::{SKIP_LIST_MAIN_FILE_ENDING, SKIP_LIST_CONFIG_FILE_ENDING, SkipList, SkipListError};
use crate::validation_json::ValidationJson;
use std::fmt::{Display, format};
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Error, Read, Write};
use std::sync::Mutex;
use crate::user_system::{User, UserPermissions, UserSystem, UserSystemError};

const DATABASE_CONFIG_ENDING: &str = ".config.json";
#[derive(Debug)]
pub enum DataBaseError {
    FileError(std::io::Error),
    CollectionError(CollectionError),
    JsonError(JsonError),
    IndexError(SkipListError),
    PermissionError,
    UserSystemError(UserSystemError),
}
impl From<std::io::Error> for DataBaseError {
    fn from(value: Error) -> Self {
        Self::FileError(value)
    }
}
impl From<JsonError> for DataBaseError {
    fn from(value: JsonError) -> Self {
        Self::JsonError(value)
    }
}
impl From<SkipListError> for DataBaseError {
    fn from(value: SkipListError) -> Self {
        Self::IndexError(value)
    }
}
impl From<UserSystemError> for DataBaseError {
    fn from(value: UserSystemError) -> Self {
        Self::UserSystemError(value)
    }
}
impl From<CollectionError> for DataBaseError {
    fn from(value: CollectionError) -> Self {
        DataBaseError::CollectionError(value)
    }
}

pub struct DataBase {
    collections: Vec<Collection>,
    key_value_store: KeyValueStore,
    index: SkipList,

    name: String,
}

impl Display for DataBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut self_as_json = JsonObject::new();
        self_as_json.insert("name".to_string(), self.name.clone().into());
        let mut collections_as_json = JsonObject::new();
        self.collections.iter().for_each(|collection| {
            collections_as_json.insert(
                collection.name.clone(),
                JsonData::infer_from_string(collection.to_string()).unwrap(),
            );
        });
        self_as_json.insert("collections".to_string(), collections_as_json.into());
        write!(f, "{}", JsonSerializer::serialize(self_as_json))
    }
}
type Res<T> = Result<T, DataBaseError>;
impl DataBase {
    pub fn save(&self) -> Res<()> {
        let mut file = Self::get_config_file(&self.name)?;
        file.write_all(self.to_string().as_bytes())?;
        Ok(())
    }
    fn get_config_file(name: &String) -> Res<File> {
        let mut open_options = OpenOptions::new();
        open_options.write(true).read(true).create(true);
        Ok(open_options.open(format!("{}{}", name, DATABASE_CONFIG_ENDING))?)
    }
    fn read_from_file(name: &String) -> Res<Self> {
        let mut file_data_as_string = String::new();
        {
            Self::get_config_file(name)?.read_to_string(&mut file_data_as_string)?;
        }
        let file_data_as_json = JsonDeserializer::deserialize(file_data_as_string)?;
        Ok(Self::try_from(file_data_as_json)?)
    }
    #[inline]
    fn check_permission<T>(&self,f:T,user_id:u128) -> Res<()> where T:FnOnce(&User)->bool
    {
        if let Ok(user)=UserSystem::get_instance().read().unwrap().get_logged_in_user(user_id) {
            if !f(&user) {
                return Err(DataBaseError::PermissionError);
            }
        } else {
            return Err(DataBaseError::PermissionError);
        }
        Ok(())
    }

    pub fn new(name: String) -> Res<Self> {
        Ok(if let Ok(db) = Self::read_from_file(&name) {
            db
        } else {
            let ret=Self {
                collections: Vec::new(),
                key_value_store: KeyValueStore::new(format!("{} storage engine", name)),
                index: SkipList::new(&format!("{} index", name))?,
                name,
            };
            ret.save()?;
            ret
        })
    }
    pub fn erase(self,user_id:u128) -> Res<()> {
        self.check_permission(User::is_admin,user_id)?;
        fs::remove_file(format!("{}{}", self.name, DATABASE_CONFIG_ENDING))?;
        fs::remove_file(format!("{} index{}", self.name, SKIP_LIST_CONFIG_FILE_ENDING))?;
        fs::remove_file(format!("{} index{}", self.name, SKIP_LIST_MAIN_FILE_ENDING))?;
        self.key_value_store.erase();
        fs::remove_file(format!("{} storage engine.{}", self.name,BLOOM_FILTER_PATH))?;
        Ok(())
    }
    pub fn create_collection(&mut self, name: String, structure: Option<ValidationJson>,user_id:u128)->Result<(),DataBaseError>{
        self.check_permission(|user| user.can_create(),user_id)?;
        let collection = Collection::new(structure, name, &mut self.key_value_store)?;
        self.collections.push(collection);
        self.save()?;
        Ok(())
    }
    fn get_collection_internal<'a>(collections: &'a Vec<Collection>, name: &'a String) -> Option<&'a Collection> {
        collections
            .iter()
            .find(|&collection| &collection.name == name)
    }
    pub fn get_mut_collection<'a>(collections: &'a mut Vec<Collection>, name: &'a String) -> Option<&'a mut Collection> {
        collections
            .iter_mut()
            .find(|collection| &collection.name == name)
    }
    pub fn insert_into_collection(&mut self, collection_name: &String,key_name:String, data: JsonObject,user_id:u128) -> Result<(),DataBaseError>{
        self.check_permission(|user| user.can_insert(collection_name),user_id)?;
        let collection = Self::get_mut_collection(&mut self.collections,collection_name).unwrap();
        collection.insert(key_name, data,&mut self.key_value_store,&mut self.index)?;
        Ok(())
    }
    pub fn get_from_collection(&self, collection_name: &String, key_name: String,user_id:u128) -> Result<Option<JsonObject>,DataBaseError>{
        self.check_permission(|user| user.can_read(collection_name),user_id)?;
        let collection = Self::get_collection_internal(&self.collections, collection_name).unwrap();
        Ok(collection.search(key_name, &self.key_value_store)?)

    }
    pub fn delete_from_collection(&mut self, collection_name: &String, key_name: String,user_id:u128) -> Result<(),DataBaseError>{
        self.check_permission(|user| user.can_delete(collection_name),user_id)?;
        let collection = Self::get_mut_collection(&mut self.collections,collection_name).unwrap();
        collection.delete(&key_name, &mut self.key_value_store, &mut self.index)?;
        Ok(())
    }
    pub fn update_collection(&mut self, collection_name: &String, key_name: String, data: JsonObject,user_id:u128) -> Result<(),DataBaseError>{
        self.check_permission(|user| user.can_update(collection_name),user_id)?;
        let collection = Self::get_mut_collection(&mut self.collections,collection_name).unwrap();
        collection.update(key_name, &mut self.key_value_store, &mut self.index,data)?;
        Ok(())
    }
    pub fn drop_collection(&mut self, name: String) {
        todo!()
    }
    pub fn signup(&mut self,username:String,password:String,user_permissions: UserPermissions)->Result<u128,DataBaseError>{
        let mut user_system = UserSystem::get_instance().write().unwrap();
        Ok(user_system.signup_with_no_type(username, password, user_permissions, &mut self.key_value_store)?)

    }
    pub fn login(&mut self,username:String,password:String)->Result<u128,DataBaseError>{
        let mut user_system = UserSystem::get_instance().write().unwrap();
        Ok(user_system.login(username, password, &mut self.key_value_store)?)
    }
    pub fn logout(&mut self,user_id:u128){
        UserSystem::get_instance().write().unwrap().logout(user_id);
    }
    pub fn get_all_collections(&self)->&Vec<Collection>{
        &self.collections
    }
    pub fn get_collection<'a>(&'a self, name: &'a String)->Option<&Collection>{
        Self::get_collection_internal(&self.collections, name)
    }
    pub fn get_all_documents_from_collection(&self, collection_name: &String, user_id:u128) ->Result<Vec<JsonObject>,DataBaseError>{
        self.check_permission(|user| user.can_read(collection_name), user_id)?;
        let collection = Self::get_collection_internal(&self.collections, collection_name).unwrap();
        Ok(collection.get_all_documents(&self.key_value_store)?)
    }
}

impl TryFrom<JsonObject> for DataBase {
    type Error = DataBaseError;

    fn try_from(value: JsonObject) -> Result<Self, Self::Error> {
        let name = value["name"].as_string();
        let index = SkipList::new(&format!("{} index", name))?;
        let mut kv = KeyValueStore::new(format!("{} storage engine", name));
        let mut collections = vec![];
        let collections_json = value["collections"].as_object()?;
        for (name, value) in collections_json.into_iter() {
            let value = value.as_object()?;
            collections.push(Collection::new(
                if value["structure"].is_null() {
                    None
                } else {
                    Some(ValidationJson::try_from(value["structure"].as_object()?)?)
                },
                name,
                &mut kv,
            )?)
        }
        Ok(Self {
            collections,
            key_value_store: kv,
            index,
            name,
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{JsonArray, JsonType};
    use crate::validation_json::{JsonConstraint, JsonValidationProperty};
    use std::cmp::Ordering;
    use crate::user_system::UserType;

    #[cfg(test)]
    fn set_up_users(data_base:&mut DataBase) -> u128 {
        if let Ok(ret) = data_base.signup("admin".to_string(),"admin".to_string(),UserType::Admin.get_permissions()) {
            ret
        }else {
            log_in(data_base)
        }
    }
    #[cfg(test)]
    fn log_in(data_base:&mut DataBase)->u128 {
        data_base.login("admin".to_string(),"admin".to_string()).unwrap()
    }
    #[test]
    fn test_db_create() {
        let mut test_collection_2_template = ValidationJson::new();
        let mut test_collection_2_template_jvp_1 =
            JsonValidationProperty::new("beni".to_string(), JsonType::Integer);

        test_collection_2_template_jvp_1
            .constraint(JsonConstraint::Unique)
            .constraint(JsonConstraint::Nullable)
            .constraint(JsonConstraint::ValueConstraint(
                JsonData::from_int(99),
                Ordering::Less,
            ));
        test_collection_2_template.add(test_collection_2_template_jvp_1);

        let mut db = DataBase::new("test db".to_string()).unwrap();
        let id=set_up_users(&mut db);
        db.create_collection("test collection 1".to_string(), None,id).expect("cant create collection 1");
        db.create_collection(
            "test collection 2".to_string(),
            Some(test_collection_2_template),
            id
        ).expect("cant create collection 2");

        println!("{}", db);

        db.erase(id).unwrap();
    }
    #[test]
    fn test_reload_db() {
        let mut test_collection_2_template = ValidationJson::new();
        let mut test_collection_2_template_jvp_1 =
            JsonValidationProperty::new("beni".to_string(), JsonType::Integer);

        test_collection_2_template_jvp_1
            .constraint(JsonConstraint::Unique)
            .constraint(JsonConstraint::Nullable)
            .constraint(JsonConstraint::ValueConstraint(
                JsonData::from_int(99),
                Ordering::Less,
            ));
        test_collection_2_template.add(test_collection_2_template_jvp_1);
        let mut test_json=JsonObject::new();
        test_json.insert("test value".to_string(), JsonData::from_string("test reload db".to_string()));
        {
            let mut db = DataBase::new("test reload db".to_string()).unwrap();
            let id=set_up_users(&mut db);
            db.create_collection("tc1".to_string(), None,id).expect("cant create collection 1");
            db.create_collection(
                "test collection 2".to_string(),
                Some(test_collection_2_template.clone()),
                id
            ).expect("cant create collection 2");

            db.insert_into_collection(&"tc1".to_string(),"test".to_string(),test_json.clone(),id).expect("cant insert");
            db.logout(id);

        }
        let mut db = DataBase::new("test reload db".to_string()).unwrap();
        let id=log_in(&mut db);
        assert_eq!(db.collections.len(), 2);
        assert!(db.collections.iter().find(|c| c.name == "tc1").is_some());
        assert!(db.collections.iter().find(|c| c.name == "test collection 2").is_some());
        assert_eq!(db.collections.iter().find(|c| c.name == "test collection 2").as_ref().unwrap().structure.as_ref().unwrap(),&test_collection_2_template);
        let test=db.get_from_collection(&"tc1".to_string(), "test".to_string(), id).expect("cant get test");
        assert!(test.is_some());
        let test=test.unwrap();
        assert_eq!(test, test_json);
        db.erase(id).unwrap();
    }
    #[test]
    fn erase_test_db() {
        let mut db=DataBase::new("api_test_db".to_string()).unwrap();
        let id=set_up_users(&mut db);
        db.erase(id).unwrap();
    }
    #[test]
    fn test_use_collections() {
        let mut test_collection_2_template = ValidationJson::new();
        let mut test_collection_2_template_jvp_1 =
            JsonValidationProperty::new("age".to_string(), JsonType::Integer);
        let mut test_collection_2_template_jvp_2 =JsonValidationProperty::new("name".to_string(), JsonType::String);
            test_collection_2_template_jvp_2
            .constraint(JsonConstraint::Nullable).constraint(JsonConstraint::Any);

        test_collection_2_template_jvp_1
            .constraint(JsonConstraint::Unique)
            .constraint(JsonConstraint::Nullable)
            .constraint(JsonConstraint::ValueConstraint(
                JsonData::from_int(99),
                Ordering::Less,
            ));
        test_collection_2_template.add(test_collection_2_template_jvp_1);
        test_collection_2_template.add(test_collection_2_template_jvp_2);

        let mut db = DataBase::new("test db".to_string()).unwrap();
        let id=set_up_users(&mut db);

        db.create_collection("collection1".to_string(), None,id).expect("collection1");
        db.create_collection(
            "collection2".to_string(),
            Some(test_collection_2_template),
            id
        ).expect("collection2");

        let range=30;
        let first_collection_name="collection1".to_string();
        let second_collection_name="collection2".to_string();

        for i in 0..range {
            let mut json=JsonObject::new();
            json.insert(format!("value_number_{}",i),i.into());
            db.insert_into_collection(&first_collection_name,i.to_string(), json,id).expect("insert failed");
        }

        for i in 0..range {
            let json=db.get_from_collection(&first_collection_name,i.to_string(),id).expect("get failed");
            assert!(json.is_some());
            assert_eq!(json.unwrap().get(&format!("value_number_{}",i)).unwrap().as_int().unwrap(),i);
        }
        for i in 0..range {
            db.delete_from_collection(&first_collection_name,i.to_string(),id).expect("delete failed");
        }

        for i in 0..range {
            let mut json=JsonObject::new();
            json.insert("age".to_string(), i.into());
            json.insert("name".to_string(), JsonData::new_null());
            db.insert_into_collection(&second_collection_name,i.to_string(), json,id).expect("insert failed");
        }
        for i in 0..range {
            let json=db.get_from_collection(&second_collection_name,i.to_string(),id).expect("get failed");
            assert!(json.is_some());
            let json=json.unwrap();
            assert_eq!(json.get(&"age".to_string()).unwrap().as_int().unwrap(), i);
            assert!(json.get(&"name".to_string()).unwrap().is_null());
        }
        for i in 0..range {
            let mut json=JsonObject::new();
            json.insert("age".to_string(), (i+range).into());
            json.insert("name".to_string(), "beni".to_string().into());
            json.insert("shani".to_string(), "this is dumb".to_string().into());
            db.update_collection(&second_collection_name,i.to_string(), json,id).expect("update failed");
        }
        for i in 0..range {
            let json=db.get_from_collection(&second_collection_name,i.to_string(),id).expect("get failed");
            assert!(json.is_some());
            let json=json.unwrap();
            assert_eq!(json.get(&"age".to_string()).unwrap().as_int().unwrap(), i+range);
            assert_eq!(json.get(&"name".to_string()).unwrap().as_string(), "beni");
            assert_eq!(json.get(&"shani".to_string()).unwrap().as_string(), "this is dumb".to_string());
        }
        for i in 0..range {
            db.delete_from_collection(&second_collection_name,i.to_string(),id).expect("delete failed");
        }

        for i in 0..range{
            let json=db.get_from_collection(&first_collection_name,i.to_string(),id).expect("get failed");
            assert!(json.is_none());
        }
        for i in 0..range{
            let json=db.get_from_collection(&second_collection_name,i.to_string(),id).expect("get failed");
            assert!(json.is_none());
        }
        for i in 0..range{
            let mut json=JsonObject::new();
            json.insert("age".to_string(), (i+100).into());
            json.insert("name".to_string(), "beni".to_string().into());
            assert!(db.insert_into_collection(&second_collection_name,i.to_string(), json,id).is_err());
        }
        db.erase(id).unwrap();
    }
    #[test]
    fn test_get_all_from_collection() {
        let collection_name="tgafc".to_string();
        let mut db = DataBase::new("test get all collections db".to_string()).unwrap();
        let id=set_up_users(&mut db);
        let range: i32= 200;
        db.create_collection(collection_name.clone(), None, id).expect("cant create collection");
        for i in 0..range {
            let mut json=JsonObject::new();
            json.insert(format!("val_num_{}", i),i.into());
            db.insert_into_collection(&collection_name, i.to_string(), json, id).expect("cant insert");
        }
        let result=db.get_all_documents_from_collection(&collection_name, id).expect("cant get all");
        let result_as_json_array=result.clone().into_iter().map(|jo| JsonData::from(jo)).collect::<Vec<JsonData>>();
        println!("result is {}",JsonSerializer::serialize_array(result_as_json_array));
        assert_eq!(result.len(), range as usize);
        for number in 0..range {
            assert!(result.iter().any(|json| {
                if let Some(data) = json.get(&format!("val_num_{}",number)) {
                    if let Ok(value) = data.as_int() {
                        return value==number;
                    }
                }
                false

            }))
        }
        db.erase(id).unwrap();
    }
}
