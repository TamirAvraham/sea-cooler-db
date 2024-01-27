use crate::encryption::EncryptionService;
use crate::json::{JsonDeserializer, JsonError, JsonObject, JsonSerializer};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use crate::key_value_store::KeyValueStore;

pub const PASS_HASH_VALUE: &str = "validator";
const PRE_DEFINED_USER_TYPES_INTERNAL_ERROR_MESSAGE:&str ="there was an internal error in the database for changing user permissions";
#[derive(Debug,PartialEq)]
enum UserSystemError {
    PermissionError,
    ImproperJsonStructure,
    UserAlreadyExists,
    UserDoesNotExist,
    UserAlreadyLoggedIn,
    IncorrectPassword,
    UserNotLoggedIn,
}
//read, insert, update, delete
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
enum CollectionPermission {
    Read,
    Insert,
    Update,
    Delete,
}
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
enum DataBasePermission {
    Create,
    Drop,
    Listen,
}

impl TryFrom<&str> for CollectionPermission {
    type Error = UserSystemError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "read" => Ok(CollectionPermission::Read),
            "insert" => Ok(CollectionPermission::Insert),
            "update" => Ok(CollectionPermission::Update),
            "delete" => Ok(CollectionPermission::Delete),
            _ => Err(UserSystemError::PermissionError),
        }
    }
}
impl TryFrom<&str> for DataBasePermission {
    type Error = UserSystemError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "create" => Ok(DataBasePermission::Create),
            "drop" => Ok(DataBasePermission::Drop),
            "listen" => Ok(DataBasePermission::Listen),
            _ => Err(UserSystemError::PermissionError),
        }
    }
}
impl From<JsonError> for UserSystemError {
    fn from(value: JsonError) -> Self {
        UserSystemError::ImproperJsonStructure
    }
}
impl Display for CollectionPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self {
            &CollectionPermission::Read => "read".to_string(),
            &CollectionPermission::Insert => "insert".to_string(),
            &CollectionPermission::Update => "update".to_string(),
            &CollectionPermission::Delete => "delete".to_string(),
        };
        write!(f, "{}", str)
    }
}
impl Display for DataBasePermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match &self {
            &DataBasePermission::Create => "create".to_string(),
            &DataBasePermission::Drop => "drop".to_string(),
            &DataBasePermission::Listen => "listen".to_string(),
        };
        write!(f, "{}", str)
    }
}
enum UserType {
    Admin,
    User,
    Guest,
}

#[derive(PartialEq,Debug, Clone)]
struct UserPermissions {
    database_permissions: Option<Vec<DataBasePermission>>,
    specific_collections:HashMap<String, Option<Vec<CollectionPermission>>>,
    all_collections:Option<Vec<CollectionPermission>>,
}
#[derive(Debug,PartialEq,Clone)]
struct User {
    username: String,
    password: Vec<u8>,
    user_permissions: UserPermissions,
}

impl UserType {
    pub fn get_permissions(&self) -> UserPermissions {
        let mut user_permissions = UserPermissions::new();
        match &self {
            &UserType::Admin => {
                user_permissions.set_db_permissions_to_all();
                user_permissions.set_collections_permissions_to_all();
            }
            &UserType::User => {
                user_permissions
                    .add_db_permission(DataBasePermission::Listen).set_collections_permissions_to_all();
            }
            &UserType::Guest => {
                user_permissions.add_collection_permission(None, CollectionPermission::Read).expect(PRE_DEFINED_USER_TYPES_INTERNAL_ERROR_MESSAGE).add_db_permission(DataBasePermission::Listen);
            },
        };
        user_permissions
    }
}

impl UserPermissions {
    pub fn new() -> Self {
        Self {
            database_permissions: Some(vec![]),
            specific_collections: HashMap::new(),
            all_collections: Some(vec![]),
        }
    }
    pub fn add_db_permission(&mut self, permission: DataBasePermission) -> &mut Self {
        if let Some(permissions) = &mut self.database_permissions {
            if let None = permissions.iter().find(|p| p == &&permission) {
                permissions.push(permission);
            }
        }
        self
    }
    pub fn add_collection_permission(
        &mut self,
        collection_name: Option<String>,
        permission: CollectionPermission,
    ) -> Result<&mut Self, UserSystemError> {
        if let Some(name) = collection_name {
            if let Some(collection_permissions) = self.specific_collections.get_mut(&name) {
                if let Some(collection_permissions) = collection_permissions {
                    if let None = collection_permissions.iter().find(|p| p == &&permission) {
                        collection_permissions.push(permission);
                    }
                }
            }else {
                self.specific_collections.insert(name, Some(vec![permission]));
            }
        } else {
            if let Some(all_collections) = &mut self.all_collections {
                if let None = all_collections.iter().find(|p| p == &&permission) {
                    all_collections.push(permission);
                }
            }
        }
        Ok(self)
    }
    pub fn set_db_permissions_to_all(&mut self) -> &mut Self {
        self.database_permissions = None;
        self
    }
    pub fn set_collections_permissions_to_all(&mut self) -> &mut Self {
        self.all_collections = None;
        self
    }
    pub fn set_collection_permissions_to_all(&mut self, collection_name: String) -> &mut Self {
        if let Some(collections_permissions) = self.specific_collections.get_mut(&collection_name) {
            *collections_permissions = None;
        } else {
            self.specific_collections.insert(collection_name, None);
        }
        self
    }

    pub fn to_json(&self) -> JsonObject {
        let mut self_as_json = JsonObject::new();

        let mut db_permissions_as_json = JsonObject::new();
        let mut all_collection_permissions_as_json = JsonObject::new();
        let mut specific_collection_permissions_as_json = JsonObject::new();

        if let Some(database_permissions) = &self.database_permissions {
            for database_permission in database_permissions {
                db_permissions_as_json.insert(database_permission.to_string(), true.into());
            }
        } else {
            db_permissions_as_json.insert("all".to_string(), true.into());
        }

        if let Some(collection_permissions) = &self.all_collections {
            for collection_permission in collection_permissions {
                all_collection_permissions_as_json.insert(collection_permission.to_string(), true.into());
            }
        }else {
            all_collection_permissions_as_json.insert("all".to_string(), true.into());
        }

        for (collection_name, permissions) in &self.specific_collections {
            let mut collection_permissions_as_json_for_collection = JsonObject::new();
            if let Some(permissions) = permissions {
                for permission in permissions {
                    collection_permissions_as_json_for_collection
                        .insert(permission.to_string(), true.into());
                }
            } else {
                collection_permissions_as_json_for_collection
                    .insert("all".to_string(), true.into());
            }
            specific_collection_permissions_as_json.insert(
                collection_name.clone(),
                collection_permissions_as_json_for_collection.into(),
            );
        }


        self_as_json.insert("db permissions".to_string(), db_permissions_as_json.into());
        self_as_json.insert(
            "all collection permissions".to_string(),
            all_collection_permissions_as_json.into(),
        );
        self_as_json.insert(
            "specific collection permissions".to_string(),
            specific_collection_permissions_as_json.into(),
        );

        self_as_json
    }

    pub fn from_json(json: JsonObject) -> Result<UserPermissions,UserSystemError> {
        let mut user_permissions = UserPermissions::new();
        let db_permissions = json.get(&"db permissions".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?.as_object()?;
        let all_collection_permissions = json.get(&"all collection permissions".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?.as_object()?;
        let specific_permissions = json.get(&"specific collection permissions".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?.as_object()?;
        if db_permissions.get(&"all".to_string()).is_some() {
            user_permissions.set_db_permissions_to_all();
        } else {
            for (key, value) in db_permissions.into_iter() {
                if value.as_bool().unwrap() {
                    user_permissions.add_db_permission(DataBasePermission::try_from(key.as_str())?);
                }
            }
        }
        if all_collection_permissions.get(&"all".to_string()).is_some() {
            user_permissions.set_collections_permissions_to_all();
        } else {
            for (key, _) in all_collection_permissions.into_iter() {
                if let Ok(permission) = CollectionPermission::try_from(key.as_str()) {
                    user_permissions.add_collection_permission(None, permission)?;
                }
            }
        }
        for (collection_name,value) in specific_permissions {
            let collection_permissions = value.as_object()?;
            if collection_permissions.get(&"all".to_string()).is_some() {
                user_permissions.set_collection_permissions_to_all(collection_name.clone());
            }else {
                for (key, _) in collection_permissions.into_iter() {
                    if let Ok(permission) = CollectionPermission::try_from(key.as_str()) {
                        user_permissions.add_collection_permission(Some(collection_name.clone()), permission)?;
                    }
                }
            }
        }


        Ok(user_permissions)
    }
}
impl User {
    fn generate_password_hash(password: String) -> Vec<u8> {
        EncryptionService::get_instance()
            .read()
            .unwrap()
            .encrypt(PASS_HASH_VALUE.to_string(), &password)
    }
    pub fn new(username: String, password: String, user_permissions: UserPermissions) -> User {
        User {
            username,
            password: Self::generate_password_hash(password),
            user_permissions,
        }
    }
    pub fn new_from_user_type(user_type: UserType, username: String, password: String) -> User {
        User::new(username, password, user_type.get_permissions())
    }
    pub fn to_json(&self) -> JsonObject {
        let mut self_as_json = JsonObject::new();
        self_as_json.insert("username".to_string(), self.username.clone().into());
        self_as_json.insert("password".to_string(), self.password.clone().into());
        self_as_json.insert(
            "permissions".to_string(),
            self.user_permissions.to_json().into(),
        );
        self_as_json
    }
    pub fn from_json(json: JsonObject) -> Result<User,UserSystemError> {
        let username = json.get(&"username".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?.as_string();
        let password:Vec<u8> = (*json.get(&"password".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?).clone().try_into()?;
        let permissions = json.get(&"permissions".to_string()).ok_or(UserSystemError::ImproperJsonStructure)?.as_object()?;

        let user_permissions = UserPermissions::from_json(permissions)?;
        Ok(User{
            username,
            password,
            user_permissions,
        })
        
    }
}

#[cfg(test)]
mod user_tests {
    use crate::json::JsonSerializer;
    use super::*;
    #[test]
    fn test_user_permissions_from_user_types() {
        let admin=User::new_from_user_type(UserType::Admin, "admin".to_string(), "123456".to_string());
        let user=User::new_from_user_type(UserType::User, "user".to_string(), "123456".to_string());
        let guest=User::new_from_user_type(UserType::Guest, "guest".to_string(), "123456".to_string());

        assert_eq!(admin.user_permissions.database_permissions,None);
        assert_eq!(admin.user_permissions.all_collections,None);
        assert_eq!(user.user_permissions.database_permissions,Some(vec![DataBasePermission::Listen]));
        assert_eq!(user.user_permissions.all_collections,None);
        assert_eq!(guest.user_permissions.database_permissions,Some(vec![DataBasePermission::Listen]));
        assert_eq!(guest.user_permissions.all_collections,Some(vec![CollectionPermission::Read]));

        println!("admin = {}",JsonSerializer::serialize(admin.to_json()));
        println!("user = {}",JsonSerializer::serialize(user.to_json()));
        println!("guest = {}",JsonSerializer::serialize(guest.to_json()));

        let new_admin = User::from_json(admin.to_json()).unwrap();
        let new_user = User::from_json(user.to_json()).unwrap();
        let new_guest = User::from_json(guest.to_json()).unwrap();

        assert_eq!(new_admin,admin);
        assert_eq!(new_user,user);
        assert_eq!(new_guest,guest);

    }
    #[test]
    fn test_custom_users() {
        let mut user_permissions = UserPermissions::new();
        let test_collection_partial_name = "partial test collection".to_string();
        let test_collection_name = "test collection".to_string();
        user_permissions
            .add_db_permission(DataBasePermission::Create)
            .add_db_permission(DataBasePermission::Listen).add_db_permission(DataBasePermission::Drop)
            .add_collection_permission(Some(test_collection_partial_name.clone()),CollectionPermission::Insert).unwrap()
            .add_collection_permission(Some(test_collection_partial_name.clone()),CollectionPermission::Update).unwrap()
            .set_collection_permissions_to_all(test_collection_name.clone());
        let user = User::new("test".to_string(),"123456".to_string(),user_permissions.clone());
        println!("user = {}",JsonSerializer::serialize(user.to_json()));
        let new_user = User::from_json(user.to_json()).unwrap();
        for permission in new_user.user_permissions.database_permissions.unwrap() {
            assert!(user_permissions.database_permissions.clone().unwrap().contains(&permission));
        }
        for permission in new_user.user_permissions.all_collections.unwrap() {
            assert!(user_permissions.all_collections.clone().unwrap().contains(&permission));
        }
        for (collection_name,permissions) in new_user.user_permissions.specific_collections.iter() {
            assert!(user_permissions.specific_collections.contains_key(collection_name));
        }
    }
}


struct UserSystem {
    logged_in_user:Option<User>,
}

impl UserSystem {
    fn new()->Self{
        Self{
            logged_in_user:None,
        }
    }
    fn get_user_from_db(&self,username:&String,kv:&KeyValueStore)->Result<User,UserSystemError>{
        let user_json = kv.search(format!("u_{}",username)).get().ok_or(UserSystemError::UserDoesNotExist)?;
        let user = User::from_json(JsonDeserializer::deserialize(user_json)?)?;
        Ok(user)
    }
    pub fn signup_using_type(&mut self, username:String, password:String, kv:&mut KeyValueStore, user_type: UserType) ->Result<(),UserSystemError>{
        self.signup_with_no_type(username,password,user_type.get_permissions(),kv)
    }
    pub fn signup_with_no_type(&mut self, username:String, password:String,user_permissions: UserPermissions, kv:&mut KeyValueStore) ->Result<(),UserSystemError>{
        if self.logged_in_user.is_some() {
            return Err(UserSystemError::UserAlreadyLoggedIn);
        }
        if let Ok(e) = self.get_user_from_db(&username,kv){
            return Err(UserSystemError::UserAlreadyExists);
        }
        let user = User::new(username.clone(),password,user_permissions);
        let res=kv.insert(format!("u_{}",username),JsonSerializer::serialize(user.to_json())).get();
        if let None = res {
            return Err(UserSystemError::UserAlreadyExists);
        }
        self.logged_in_user = Some(user);
        Ok(())
    }
    pub fn guest_login(&mut self,username:String,key_value_store: &KeyValueStore) ->Result<(),UserSystemError> {
        if self.logged_in_user.is_some() {
            return Err(UserSystemError::UserAlreadyLoggedIn);
        }
        if let Ok(_) = self.get_user_from_db(&username,key_value_store){
            return Err(UserSystemError::UserAlreadyExists);
        }
        let user = User::new_from_user_type(UserType::Guest, username, "very password yes yes".to_string());
        self.logged_in_user = Some(user);
        Ok(())
    }
    pub fn login(&mut self,username:String,password:String,key_value_store: &KeyValueStore) ->Result<(),UserSystemError> {
        if self.logged_in_user.is_some() {
            return Err(UserSystemError::UserAlreadyLoggedIn);
        }
        let user = self.get_user_from_db(&username,key_value_store)?;
        if user.password != User::generate_password_hash(password) {
            return Err(UserSystemError::IncorrectPassword);
        }
        self.logged_in_user = Some(user);
        Ok(())
    }
    pub fn logout(&mut self) ->Result<(),UserSystemError> {
        if self.logged_in_user.is_none() {
            return Err(UserSystemError::UserNotLoggedIn);
        }
        self.logged_in_user = None;
        Ok(())
    }
    pub fn get_logged_in_user(&self) ->Option<User>{
        self.logged_in_user.clone()
    }

    pub fn modify_user_permissions(&mut self,username:String,user_permissions: UserPermissions,kv:&mut KeyValueStore) ->Result<(),UserSystemError> {
        let user = self.get_user_from_db(&username, kv)?;
        let new_user = User{username:user.username, password:user.password, user_permissions};
        kv.update(format!("u_{}", username), JsonSerializer::serialize(new_user.to_json())).get();
        Ok(())
    }
}

impl User{
    pub fn is_admin(&self)->bool {
        self.user_permissions.database_permissions.is_some() && self.user_permissions.all_collections.is_some() && self.user_permissions.specific_collections.is_empty()
    }
    fn can_do_for_collection(&self,collection_name:&String,prem:&CollectionPermission)->bool {
        if let Some(collection_settings) = self.user_permissions.specific_collections.get(collection_name) {
            if let Some(collection_settings) = collection_settings {
                collection_settings.contains(prem)
            }else {
                true
            }
        }else {
            if let Some(collections_settings) = &self.user_permissions.all_collections {
                collections_settings.contains(prem)
            }else {
                true
            }
        }
    }
    fn can_do_for_db(&self,prem:&DataBasePermission)->bool {
        if let Some(db_settings) = &self.user_permissions.database_permissions {
            db_settings.contains(prem)
        }else {
            true
        }
    }
    pub fn can_insert(&self,collection_name:&String)->bool {
        self.can_do_for_collection(collection_name,&CollectionPermission::Insert)
    }
    pub fn can_update(&self,collection_name:&String)->bool {
        self.can_do_for_collection(collection_name,&CollectionPermission::Update)
    }
    pub fn can_delete(&self,collection_name:&String)->bool {
        self.can_do_for_collection(collection_name,&CollectionPermission::Delete)
    }
    pub fn can_read(&self,collection_name:&String)->bool {
        self.can_do_for_collection(collection_name,&CollectionPermission::Read)
    }
    pub fn can_listen(&self)->bool {
        self.can_do_for_db(&DataBasePermission::Listen)
    }
    pub fn can_create(&self)->bool {
        self.can_do_for_db(&DataBasePermission::Create)
    }
    pub fn can_drop(&self)->bool {
        self.can_do_for_db(&DataBasePermission::Drop)
    }
}

#[cfg(test)]
mod user_system_tests {
    use super::*;
    #[cfg(test)]
    fn create_kv_for_user_tests() -> KeyValueStore {
        let mut kv = KeyValueStore::new("user system tests".to_string());


        kv
    }
    #[test]
    fn test_user_signup() {
        let mut kv = create_kv_for_user_tests();
        let mut user_system = UserSystem::new();
        let user_type = UserType::Admin;
        let username = "XXXX".to_string();
        let password = "XXXX".to_string();

        assert_eq!(user_system.login(username.clone(), password.clone(), &kv),Err(UserSystemError::UserDoesNotExist));
        assert_eq!(user_system.signup_using_type(username.clone(), password.clone(), &mut kv, user_type),Ok(()));
        assert_ne!(user_system.get_logged_in_user(),None);
        assert_eq!(user_system.get_logged_in_user().unwrap().username,username);
        user_system.logout().unwrap();
        assert_eq!(user_system.get_logged_in_user(),None);
        assert_eq!(user_system.login(username.clone(), password.clone(), &kv),Ok(()));
        assert_eq!(user_system.get_logged_in_user().unwrap().username,username);
        kv.erase()
    }
}