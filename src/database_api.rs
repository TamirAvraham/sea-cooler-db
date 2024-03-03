use crate::collection::CollectionError;
use crate::database::{DataBase, DataBaseError};
use crate::http_parser::{HttpMethod, HttpRequest};
use crate::http_server::{HttpResponse, HttpServer, HttpStatusCode};
use crate::json::{JsonArray, JsonData, JsonDeserializer, JsonObject, JsonSerializer};
use crate::user_system::{UserPermissions, UserSystemError};
use crate::validation_json::{JsonValidationError, ValidationJson};
use std::cell::RefCell;
use std::sync::{Arc, RwLock};

fn response_from_string(status_code: HttpStatusCode, message: &str) -> HttpResponse {
    let mut json = JsonObject::new();
    json.insert(
        "message".to_string(),
        JsonData::from_string(message.to_string()),
    );
    HttpResponse::new_from_json(status_code, JsonSerializer::serialize(json))
}
fn internal_error_response(msg: &str) -> HttpResponse {
    response_from_string(HttpStatusCode::OK, msg)
}
fn default_internal_error_response() -> HttpResponse {
    internal_error_response("Internal Server Error")
}
fn invalid_request_response(msg: &str) -> HttpResponse {
    response_from_string(HttpStatusCode::BadRequest, msg)
}
fn ok_response() -> HttpResponse {
    HttpResponse::new(HttpStatusCode::OK, None)
}
//---------------------------------------- Users part of the api -----------------------------------------------------//
const LOGIN_URL: &str = "/login";
const LOGIN_METHOD: HttpMethod = HttpMethod::POST;
const LOGOUT_URL: &str = "/logout";
const LOGOUT_METHOD: HttpMethod = HttpMethod::POST;
const SIGNUP_URL: &str = "/register";
const SIGNUP_METHOD: HttpMethod = HttpMethod::POST;
static mut DATABASE: Option<RwLock<DataBase>> = None;
fn get_data_base() -> Option<&'static RwLock<DataBase>> {
    unsafe { DATABASE.as_ref() }
}
fn get_login_args(http_request: &HttpRequest) -> Option<(String, String)> {
    Some((
        http_request.params.get("username")?.clone(),
        http_request.params.get("password")?.clone(),
    ))
}
fn login(http_request: HttpRequest) -> Option<HttpResponse> {
    Some(
        if let Some((username, password)) = get_login_args(&http_request) {
            if let Some(db) = get_data_base() {
                let login_result = db.write().unwrap().login(username, password);
                match login_result {
                    Ok(token) => {
                        let mut json = JsonObject::new();
                        json.insert("user_id".to_string(), token.into());
                        HttpResponse::new_from_json(
                            HttpStatusCode::OK,
                            JsonSerializer::serialize(json),
                        )
                    }
                    Err(err) => match err {
                        DataBaseError::UserSystemError(err) => match err {
                            UserSystemError::UserAlreadyLoggedIn => {
                                invalid_request_response("User already logged in")
                            }
                            UserSystemError::UserDoesNotExist
                            | UserSystemError::IncorrectPassword => {
                                invalid_request_response("Invalid Credentials")
                            }
                            _ => default_internal_error_response(),
                        },
                        _ => default_internal_error_response(),
                    },
                }
            } else {
                internal_error_response("Db was not created")
            }
        } else {
            invalid_request_response("Invalid Arguments")
        },
    )
}
fn logout(http_request: HttpRequest) -> Option<HttpResponse> {
    Some(if let Ok(user_id) = get_user_id(&http_request) {
        if let Some(db) = get_data_base() {
            db.write().unwrap().logout(user_id);
            ok_response()
        } else {
            internal_error_response("Db was not created")
        }
    } else {
        invalid_request_response("Invalid Arguments: User Id Missing")
    })
}
fn get_signup_args(http_request: &HttpRequest) -> Option<(String, String, UserPermissions)> {
    let body = http_request.body.clone()?;
    let body = JsonDeserializer::deserialize(body).ok()?;
    let username = body.get(&"username".to_string())?.as_string();
    let password = body.get(&"password".to_string())?.as_string();
    let permissions =
        UserPermissions::from_json(body.get(&"permissions".to_string())?.as_object().ok()?).ok()?;
    Some((username, password, permissions))
}

fn signup(http_request: HttpRequest) -> Option<HttpResponse> {
    Some(
        if let Some((username, password, permissions)) = get_signup_args(&http_request) {
            if let Some(db) = get_data_base() {
                let signup_result = db.write().unwrap().signup(username, password, permissions);
                match signup_result {
                    Ok(user_id) => {
                        let mut json = JsonObject::new();
                        json.insert("user_id".to_string(), user_id.into());
                        HttpResponse::new_from_json(
                            HttpStatusCode::OK,
                            JsonSerializer::serialize(json),
                        )
                    }
                    Err(err) => match err {
                        DataBaseError::UserSystemError(err) => match err {
                            UserSystemError::UserAlreadyLoggedIn => {
                                invalid_request_response("User already logged in")
                            }
                            UserSystemError::UserAlreadyExists => {
                                invalid_request_response("User already exists")
                            }
                            _ => default_internal_error_response(),
                        },
                        _ => default_internal_error_response(),
                    },
                }
            } else {
                internal_error_response("Db was not created")
            }
        } else {
            invalid_request_response("Invalid Arguments")
        },
    )
}
//----------------------------------------- Collection part of the api -----------------------------------------------//
fn get_user_id(http_request: &HttpRequest) -> Result<u128, HttpResponse> {
    if let Some(user_id) = http_request.get_param("user_id") {
        if let Ok(ret) = user_id.parse::<u128>() {
            return Ok(ret);
        }
    }
    Err(invalid_request_response(
        "Invalid Arguments: User Id Missing In Cookie",
    ))
}
const CREATE_NEW_COLLECTION_URL: &str = "/create_new_collection";
const CREATE_NEW_COLLECTION_METHOD: HttpMethod = HttpMethod::POST;
pub fn create_new_collection(http_request: HttpRequest) -> Option<HttpResponse> {
    Some(if http_request.body_is_json() {
        let body = http_request.body.clone().unwrap(); // body is json checks if there is a body
        let body = JsonDeserializer::deserialize(body);
        if let Err(_) = body {
            invalid_request_response("Body was Not Proper Json");
        }
        let body = body.unwrap();
        let collection_name = body.get(&"collection_name".to_string());
        if let None = collection_name {
            return Some(invalid_request_response("Collection name was not found"));
        }
        let user_id = match get_user_id(&http_request) {
            Ok(user_id) => user_id,
            Err(ret) => return Some(ret),
        };
        let collection_name = collection_name.unwrap().as_string();
        if let Some(structure_as_json_data) = body.get(&"collection_structure".to_string()) {
            if let Ok(structure_as_json_object) = structure_as_json_data.as_object() {
                if let Ok(structure) = ValidationJson::try_from(structure_as_json_object) {
                    return if let Some(db) = get_data_base() {
                        match db.write().unwrap().create_collection(
                            collection_name,
                            Some(structure),
                            user_id,
                        ) {
                            Ok(_) => Some(ok_response()),
                            Err(e) => Some(send_db_error_msg(e, "create new collection")),
                        }
                    } else {
                        Some(internal_error_response("Db was not created"))
                    };
                }
            }
            invalid_request_response("Collection structure was not formatted correctly")
        } else {
            if let Some(db) = get_data_base() {
                if let Ok(_) = db
                    .write()
                    .unwrap()
                    .create_collection(collection_name, None, user_id)
                {
                    ok_response()
                } else {
                    default_internal_error_response()
                }
            } else {
                internal_error_response("Db was not created")
            }
        }
    } else {
        invalid_request_response("Collection structure was not found")
    })
}
const COLLECTIONS_INFO_URL: &str = "/collections";
const COLLECTIONS_INFO_METHOD: HttpMethod = HttpMethod::GET;
pub fn collections_info(http_request: HttpRequest) -> Option<HttpResponse> {
    return Some(match http_request.get_param("collection_name") {
        None => {
            if let Some(db) = get_data_base() {
                let collections = db
                    .read()
                    .unwrap()
                    .get_all_collections()
                    .iter()
                    .map(|collection| {
                        let mut json = JsonObject::new();
                        json.insert(
                            collection.name.clone(),
                            JsonData::from(collection.to_json()),
                        );
                        json
                    })
                    .fold(JsonArray::new(), |mut arr, collection_as_json| {
                        arr.push(JsonData::from(collection_as_json));
                        arr
                    });

                let mut json = JsonObject::new();
                json.insert("collections".to_string(), collections.into());

                HttpResponse::new_from_json(HttpStatusCode::OK, JsonSerializer::serialize(json))
            } else {
                internal_error_response("Db not created")
            }
        }
        Some(collection_name) => {
            if let Some(db) = get_data_base() {
                let db = db.read().unwrap();
                let collection_name = collection_name.to_string();
                let collection = db.get_collection(&collection_name);
                if let Some(collection) = collection {
                    let mut json = JsonObject::new();
                    json.insert("name".to_string(), JsonData::from(collection.to_json()));
                    HttpResponse::new_from_json(HttpStatusCode::OK, JsonSerializer::serialize(json))
                } else {
                    invalid_request_response("Collection not found")
                }
            } else {
                internal_error_response("Db not created")
            }
        }
    });
}
//--------------------------------------- Collection CRUD part of the api --------------------------------------------//
fn get_collection_name(http_request: &HttpRequest) -> Result<String, HttpResponse> {
    if let Some(collection_name) = http_request.get_param("collection_name") {
        return Ok(collection_name.to_string());
    }
    Err(invalid_request_response(
        "Invalid Arguments: Collection Name Missing",
    ))
}
fn send_db_error_msg(err: DataBaseError, operation: &str) -> HttpResponse {
    return match err {
        DataBaseError::CollectionError(CollectionError::InvalidData(
            JsonValidationError::IsNull,
        )) => invalid_request_response("Json was null"),
        DataBaseError::CollectionError(CollectionError::InvalidData(
            JsonValidationError::ValueDoesNotMeetConstraint(x, y, o),
        )) => invalid_request_response(
            format!("Value {} does not meet constraint {} {:?}", x, y, o).as_str(),
        ),
        DataBaseError::CollectionError(CollectionError::InvalidData(
            JsonValidationError::MissingProperty(prop),
        )) => invalid_request_response(format!("Missing property {}", prop).as_str()),
        DataBaseError::CollectionError(CollectionError::InvalidData(
            JsonValidationError::IncorrectType(var_name, var_type, needed_type),
        )) => invalid_request_response(
            format!(
                "Incorrect type {} expected {} at {} ",
                var_type, needed_type, var_name
            )
            .as_str(),
        ),
        DataBaseError::CollectionError(CollectionError::InvalidData(
            JsonValidationError::ValueAllReadyExists(v),
        )) => invalid_request_response(format!("Value {} all ready exists", v).as_str()),
        DataBaseError::JsonError(_) => invalid_request_response("Json was not formatted correctly"),
        DataBaseError::PermissionError
        | DataBaseError::UserSystemError(UserSystemError::PermissionError) => {
            invalid_request_response(
                format!(
                    "User does not have permission to {} into this collection",
                    operation
                )
                .as_str(),
            )
        }
        DataBaseError::UserSystemError(UserSystemError::UserNotLoggedIn) => {
            invalid_request_response("User is not logged in")
        }
        DataBaseError::CollectionError(_)
        | DataBaseError::UserSystemError(_)
        | DataBaseError::IndexError(_)
        | DataBaseError::FileError(_) => default_internal_error_response(),
    };
}

const COLLECTION_CRUD_URL: &str = "/collection";
fn insert_document_into_collection(http_request: HttpRequest) -> Option<HttpResponse> {
    if !http_request.body_is_json() {
        return Some(invalid_request_response("Body was not json"));
    }
    let body = http_request.body.clone().unwrap();
    let body = JsonDeserializer::deserialize(body);
    if let Err(_) = body {
        return Some(invalid_request_response("Body was not proper json"));
    }
    let body = body.unwrap();
    let collection_name = match get_collection_name(&http_request) {
        Ok(collection_name) => collection_name,
        Err(ret) => return Some(ret),
    };
    let user_id = match get_user_id(&http_request) {
        Ok(user_id) => user_id,
        Err(ret) => return Some(ret),
    };
    let document_name = body.get(&"document_name".to_string());
    if let None = document_name {
        return Some(invalid_request_response("Document name was not found"));
    }
    let document_name = document_name.unwrap().as_string();
    let data = body.get(&"data".to_string());
    if let None = data {
        return Some(invalid_request_response("Document was not found"));
    }
    let data = data.unwrap();
    if let Ok(data) = data.as_object() {
        if let Some(db) = get_data_base() {
            if let Err(err) = db.write().unwrap().insert_into_collection(
                &collection_name,
                document_name,
                data,
                user_id,
            ) {
                Some(send_db_error_msg(err, "insert"))
            } else {
                Some(ok_response())
            }
        } else {
            Some(internal_error_response("Db was not created"))
        }
    } else {
        Some(invalid_request_response(
            "Json didnt not have properly formatted data",
        ))
    }
}
fn update_document_in_collection(http_request: HttpRequest) -> Option<HttpResponse> {
    if !http_request.body_is_json() {
        return Some(invalid_request_response("Body was not json"));
    }
    let body = http_request.body.clone().unwrap();
    let body = JsonDeserializer::deserialize(body);
    if let Err(_) = body {
        return Some(invalid_request_response("Body was not proper json"));
    }
    let body = body.unwrap();
    let collection_name = match get_collection_name(&http_request) {
        Ok(collection_name) => collection_name,
        Err(ret) => return Some(ret),
    };
    let user_id = match get_user_id(&http_request) {
        Ok(user_id) => user_id,
        Err(ret) => return Some(ret),
    };
    let document_name = body.get(&"document_name".to_string());
    if let None = document_name {
        return Some(invalid_request_response("Document name was not found"));
    }
    let document_name = document_name.unwrap().as_string();
    let data = body.get(&"data".to_string());
    if let None = data {
        return Some(invalid_request_response("Document was not found"));
    }
    let data = data.unwrap();
    if let Ok(data) = data.as_object() {
        if let Some(db) = get_data_base() {
            if let Err(err) = db.write().unwrap().update_collection(
                &collection_name,
                document_name,
                data,
                user_id,
            ) {
                Some(send_db_error_msg(err, "update"))
            } else {
                Some(ok_response())
            }
        } else {
            Some(internal_error_response("Db was not created"))
        }
    } else {
        Some(invalid_request_response(
            "Json didnt not have properly formatted data",
        ))
    }
}
fn delete_document_in_collection(http_request: HttpRequest) -> Option<HttpResponse> {
    let collection_name = match get_collection_name(&http_request) {
        Ok(collection_name) => collection_name,
        Err(ret) => return Some(ret),
    };
    let user_id = match get_user_id(&http_request) {
        Ok(user_id) => user_id,
        Err(ret) => return Some(ret),
    };
    let document_name = match http_request.get_param("document_name") {
        None => return Some(invalid_request_response("document name param missing")),
        Some(value) => value.to_string(),
    };
    if let Some(db) = get_data_base() {
        if let Err(err) =
            db.write()
                .unwrap()
                .delete_from_collection(&collection_name, document_name, user_id)
        {
            Some(send_db_error_msg(err, "delete"))
        } else {
            Some(ok_response())
        }
    } else {
        Some(internal_error_response("Db was not created"))
    }
}
fn get_all_collection_documents(collection_name: &String, user_id: u128) -> HttpResponse {
    if let Some(db) = get_data_base() {
        match db
            .read()
            .unwrap()
            .get_all_documents_from_collection(&collection_name, user_id)
        {
            Ok(value) => {
                let mut json = JsonObject::new();
                json.insert("documents".to_string(), value.into());
                HttpResponse::new_from_json(HttpStatusCode::OK, JsonSerializer::serialize(json))
            }

            Err(err) => send_db_error_msg(err, "read"),
        }
    } else {
        internal_error_response("Db was not created")
    }
}
fn read_document_from_collection(http_request: HttpRequest) -> Option<HttpResponse> {
    let collection_name = match get_collection_name(&http_request) {
        Ok(collection_name) => collection_name,
        Err(ret) => return Some(ret),
    };
    let user_id = match get_user_id(&http_request) {
        Ok(user_id) => user_id,
        Err(ret) => return Some(ret),
    };
    let document_name = match http_request.get_param("document_name") {
        None => return Some(get_all_collection_documents(&collection_name, user_id)),
        Some(value) => value.to_string(),
    };
    if let Some(db) = get_data_base() {
        match db
            .read()
            .unwrap()
            .get_from_collection(&collection_name, document_name, user_id)
        {
            Ok(Some(value)) => Some(HttpResponse::new_from_json(
                HttpStatusCode::OK,
                JsonSerializer::serialize(value),
            )),
            Ok(None) => Some(HttpResponse::new(HttpStatusCode::NotFound, None)),
            Err(err) => Some(send_db_error_msg(err, "read")),
        }
    } else {
        Some(internal_error_response("Db was not created"))
    }
}

//--------------------------------------------------------------------------------------------------------------------//
fn set_up_db(db_name: String) -> Option<&'static RwLock<DataBase>> {
    let db = DataBase::new(db_name);
    unsafe {
        if let Ok(db) = db {
            DATABASE = Some(RwLock::new(db));
        }
    }
    get_data_base()
}
pub fn start_db_api(db_name: String) {
    if let None = set_up_db(db_name) {
        panic!("Could not create db");
    }
    let mut db_server = HttpServer::new_localhost(80);
    //user parts of the api
    db_server.add_route(LOGIN_METHOD, LOGIN_URL, login);
    db_server.add_route(LOGOUT_METHOD, LOGOUT_URL, logout);
    db_server.add_route(SIGNUP_METHOD, SIGNUP_URL, signup);

    //collection parts of the api
    db_server.add_route(
        CREATE_NEW_COLLECTION_METHOD,
        CREATE_NEW_COLLECTION_URL,
        create_new_collection,
    );
    db_server.add_route(
        HttpMethod::OPTIONS,
        CREATE_NEW_COLLECTION_URL,
        |request| Some(HttpResponse::new_options_response_default()),
    );
    db_server.add_route(
        COLLECTIONS_INFO_METHOD,
        COLLECTIONS_INFO_URL,
        collections_info,
    );

    //collection CRUD parts of the api
    db_server.add_route(
        HttpMethod::POST,
        COLLECTION_CRUD_URL,
        insert_document_into_collection,
    );
    db_server.add_route(
        HttpMethod::PUT,
        COLLECTION_CRUD_URL,
        update_document_in_collection,
    );
    db_server.add_route(
        HttpMethod::DELETE,
        COLLECTION_CRUD_URL,
        delete_document_in_collection,
    );
    db_server.add_route(
        HttpMethod::GET,
        COLLECTION_CRUD_URL,
        read_document_from_collection,
    );
    db_server.add_route(
        HttpMethod::OPTIONS,
        COLLECTION_CRUD_URL,
        |request| Some(HttpResponse::new_options_response_default()),
    );

    db_server.listen();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user_system::{UserSystem, UserType};
    //static error
    #[test]
    fn test_user_api() {
        start_db_api("api_test_db".to_string());
        loop {}
    }
    #[test]
    fn test_database_saving() {
        let db_name = "test_db".to_string();
        let collection_name = "tc".to_string();
        let doc_name = "doc1".to_string();

        let mut test_json = JsonObject::new();

        let username = "XXXX".to_string();
        let password = "XXXX".to_string();

        test_json.insert(
            "test".to_string(),
            JsonData::from_string("test".to_string()),
        );

        {
            let db = set_up_db(db_name.clone()).expect("Could not create db");
            let mut db = db.write().unwrap();
            let user_1 = db
                .signup(
                    username.clone(),
                    password.clone(),
                    UserType::Admin.get_permissions(),
                )
                .unwrap();
            db.create_collection(collection_name.clone(), None, user_1)
                .unwrap();
            db.insert_into_collection(
                &collection_name,
                doc_name.clone(),
                test_json.clone(),
                user_1,
            )
            .unwrap();
            unsafe {
                DATABASE = None;
            }
            UserSystem::get_instance().write().unwrap().logout(user_1);
        }

        let db = set_up_db(db_name.clone()).expect("Could not create db");
        let mut db = db.write().unwrap();
        let user_2 = db.login(username.clone(), password.clone()).unwrap();
        let doc = db
            .get_from_collection(&collection_name, doc_name.clone(), user_2)
            .unwrap()
            .unwrap();
        assert_eq!(doc, test_json);
    }
    #[test]
    fn delete_test_db() {
        let mut db = DataBase::new("test_db".to_string()).expect("Could not create db");
        db.login("XXXX".to_string(), "XXXX".to_string()).unwrap();
        db.erase(1).unwrap();
    }
}
