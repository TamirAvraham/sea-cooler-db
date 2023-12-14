use crate::{validation_json::{ValidationJson, JsonValidationError}, key_value_store::{KeyValueStore, KeyValueError}, json::{JsonObject, JsonSerializer}};
enum CollectionError {
    InvalidData(JsonValidationError),
    KeyValueError(KeyValueError),
}
impl From<JsonValidationError> for CollectionError {
    fn from(item: JsonValidationError) -> Self {
        CollectionError::InvalidData(item)
    }
}
impl From<KeyValueError> for CollectionError {
    fn from(item: KeyValueError) -> Self {
        CollectionError::KeyValueError(item)
    }
}
struct Collection {
    name:String,
    structure:Option<ValidationJson>,
    
}
/*
todo: 
implement ranged search
decide on a data structure for indexes 
implemnt index data structure
 */

impl Collection {
    pub fn new(structure:Option<ValidationJson>,name:String) -> Self{
        Self { name, structure,  }
    }
    pub fn new_structured(structure:ValidationJson,name:String) -> Self{
        Self{ name, structure: Some(structure),  }
    }
    pub fn new_unstructured(name:String) -> Self{
        Self{ name, structure: None,  }
    }
    fn search_index(&self,filed_name:&String,filed_value:&String){

    }
    fn update_index(&self,filed_name:&String,filed_value:&String) {
        
    }
    pub fn insert(&mut self,record_name:String,record_value:JsonObject,kv:&mut KeyValueStore) -> Result<(),CollectionError>{
        let index_fields;
        if let Some(structure) = &self.structure {
            structure.validate(&record_value)?;

            index_fields=structure.get_all_props().iter().map(|vp| format!("{}_{}_{}",self.name,vp.name,record_value[&vp.name].as_string())).collect::<Vec<String>>();

            for unique in structure.get_all_unique_props(){
                
            }
        }
        kv.insert(format!("{}_{}",self.name,record_name), JsonSerializer::serialize(record_value));
        Ok(())
    }
}

