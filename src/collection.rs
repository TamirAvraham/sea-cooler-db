use crate::json::{JsonData, JsonDeserializer, JsonError};
use crate::skip_list::{SkipList, SkipListError};
use crate::{
    json::{JsonObject, JsonSerializer},
    key_value_store::{KeyValueError, KeyValueStore},
    validation_json::{JsonValidationError, ValidationJson},
};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
#[derive(Debug)]
pub enum CollectionError {
    InvalidData(JsonValidationError),
    KeyValueError(KeyValueError),
    IndexError(SkipListError),
    InvalidJson(JsonError),
    InternalError
}
impl From<JsonValidationError> for CollectionError {
    fn from(item: JsonValidationError) -> Self {
        CollectionError::InvalidData(item)
    }
}

impl From<JsonError> for CollectionError {
    fn from(value: JsonError) -> Self {
        Self::InvalidJson(value)
    }
}
impl From<SkipListError> for CollectionError {
    fn from(value: SkipListError) -> Self {
        Self::IndexError(value)
    }
}
impl From<KeyValueError> for CollectionError {
    fn from(item: KeyValueError) -> Self {
        CollectionError::KeyValueError(item)
    }
}
pub struct Collection {
    pub name: String,
    pub structure: Option<ValidationJson>,
}
/*
todo:
implement ranged search
decide on a data structure for indexes
implemnt index data structure
 */
impl Display for Collection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut self_as_json = JsonObject::new();
        self_as_json.insert("structure".to_string(), match &self.structure {
            None => { JsonData::new_null() }
            Some(structure) => { JsonData::infer_from_string(structure.to_string()).unwrap() }
        });
        write!(f, "{}", JsonSerializer::serialize(self_as_json))
    }
}
impl Collection {
    pub fn new(structure: Option<ValidationJson>, name: String) -> Self {
        Self { name, structure }
    }
    pub fn new_structured(structure: ValidationJson, name: String) -> Self {
        Self {
            name,
            structure: Some(structure),
        }
    }
    pub fn new_unstructured(name: String) -> Self {
        Self {
            name,
            structure: None,
        }
    }
    fn search_index(
        &self,
        filed_name: &String,
        filed_value: &String,
        index: &SkipList,
    ) -> Result<Option<Vec<usize>>, CollectionError> {
        Ok(index.search(&format!("{}_{}_{}", self.name, filed_name, filed_value))?)
    }
    fn update_index(
        &self,
        filed_name: &String,
        filed_value: &String,
        index: &mut SkipList,
        value_locations: Vec<usize>,
    ) -> Result<(), CollectionError> {
        let prev_values = self.search_index(filed_name, filed_value, index)?;
        if let Some(mut values) = prev_values {
            index.insert(
                format!("{}_{}_{}", self.name, filed_name, filed_value),
                values
                    .into_iter()
                    .chain(value_locations.into_iter())
                    .collect::<HashSet<usize>>()
                    .into_iter()
                    .collect::<Vec<usize>>(),
            )?;
        } else {
            index.insert(
                format!("{}_{}_{}", self.name, filed_name, filed_value),
                value_locations,
            )?;
        }
        Ok(())
    }
    fn delete_index(
        &self,
        filed_name: &String,
        filed_value: &String,
        index: &mut SkipList,
        values: Vec<usize>,
    ) -> Result<(), CollectionError> {
        let prev_values = self.search_index(filed_name, filed_value, index)?;
        if let Some(mut prev_values) = prev_values {
            prev_values.retain(|x| !values.contains(x));
            if prev_values.is_empty() {
                index.delete(&format!("{}_{}_{}", self.name, filed_name, filed_value))?;
            } else {
                index.insert(
                    format!("{}_{}_{}", self.name, filed_name, filed_value),
                    prev_values,
                )?;
            }
        }
        Ok(())
    }
    pub fn insert(
        &mut self,
        record_name: String,
        record_value: JsonObject,
        kv: &mut KeyValueStore,
        index: &mut SkipList,
    ) -> Result<(), CollectionError> {
        let mut record_value_clone = None;
        if let Some(structure) = &self.structure {
            //this scope validates the data
            structure.validate(&record_value)?;
            for unique in structure.get_all_unique_props() {
                if let Some(values) =
                    self.search_index(&unique.name, &record_value[&unique.name].as_string(), index)?
                {
                    return Err(CollectionError::InvalidData(
                        JsonValidationError::ValueAllReadyExists(unique.name.clone()),
                    ));
                }
            }
            record_value_clone = Some(record_value.clone());
        }

        let value_location = kv.insert(
            format!("{}_{}", self.name, record_name),
            JsonSerializer::serialize(record_value),
        );

        if let Some(structure) = &self.structure {
            // update index

            if let Some(value_location) = value_location.get() {
                let record_value = record_value_clone.unwrap();
                for validation_property in structure.get_all_props() {
                    self.update_index(
                        &validation_property.name,
                        &record_value[&validation_property.name].as_string(),
                        index,
                        vec![value_location],
                    )?;
                }
            }
        }else {
            value_location.get().ok_or(CollectionError::InternalError)?;
        }

        Ok(())
    }
    pub fn search(
        &self,
        record_name: String,
        kv: &KeyValueStore,
    ) -> Result<Option<JsonObject>, CollectionError> {
        if let Some(search_result) = kv.search(format!("{}_{}", self.name, record_name)).get() {
            Ok(Some(JsonDeserializer::deserialize(search_result)?))
        } else {
            Ok(None)
        }
    }
    pub fn delete(
        &mut self,
        record_name: &String,
        kv: &mut KeyValueStore,
        index: &mut SkipList,
    ) -> Result<(), CollectionError> {
        if let Some(structure) = &self.structure {
            // update index
            let value_locations = self.search_index(&self.name, record_name, index)?;
            if let Some(value_locations) = value_locations {
                for validation_property in structure.get_all_props() {
                    self.delete_index(
                        &validation_property.name,
                        &record_name,
                        index,
                        value_locations.clone(),
                    )?;
                }
            }
        }
        kv.delete(format!("{}_{}", self.name, record_name));
        Ok(())
    }
    pub fn update(
        &mut self,
        record_name: String,
        kv: &mut KeyValueStore,
        index: &mut SkipList,
        new_record: JsonObject,
    ) -> Result<(), CollectionError> {
        let mut record_value_clone = None;
        if let Some(structure) = &self.structure {
            //this scope validates the data
            structure.validate(&new_record)?;
            for unique in structure.get_all_unique_props() {
                if let Some(values) =
                    self.search_index(&unique.name, &new_record[&unique.name].as_string(), index)?
                {
                    return Err(CollectionError::InvalidData(
                        JsonValidationError::ValueAllReadyExists(unique.name.clone()),
                    ));
                }
            }
            record_value_clone = Some(new_record.clone());
        }

        let value_location = kv.update(
            format!("{}_{}", self.name, record_name),
            JsonSerializer::serialize(new_record),
        );

        if let Some(structure) = &self.structure {
            // update index
            if let Some((_, value_location)) = value_location.get() {
                let new_record = record_value_clone.unwrap();
                for validation_property in structure.get_all_props() {
                    self.update_index(
                        &validation_property.name,
                        &new_record[&validation_property.name].as_string(),
                        index,
                        vec![value_location],
                    )?;
                }
            }
        }

        Ok(())
    }
}
