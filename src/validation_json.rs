use std::fmt::Display;
use std::{cmp::Ordering, collections::HashSet};

use crate::json::{JsonData, JsonError, JsonObject, JsonSerializer, JsonType};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonValidationError {
    IsNull,
    ValueDoesNotMeetConstraint(JsonData, JsonData, Ordering),
    MissingProperty(String),
    IncorrectType(String, JsonType, JsonType),
    ValueAllReadyExists(String),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum JsonConstraint {
    Nullable,
    ValueConstraint(JsonData, Ordering),
    Unique,
    Any,
}

#[derive(PartialEq, Debug, Clone)]
pub struct JsonValidationProperty {
    pub name: String,
    pub data_type: JsonType,
    pub constraints: HashSet<JsonConstraint>,
}
#[derive(Debug, PartialEq, Clone)]
pub struct ValidationJson {
    props: Vec<JsonValidationProperty>,
}
impl JsonValidationProperty {
    pub fn new(name: String, data_type: JsonType) -> Self {
        Self {
            name,
            data_type,
            constraints: HashSet::new(),
        }
    }
    /// # Description
    /// function adds constraint to the property
    ///
    /// # Panics
    /// * if the data type of the property and the constraint do not match
    /// # Arguments
    ///
    /// * `constraint`: value constraint to be added to the property
    ///
    /// returns: JsonValidationProperty
    ///
    pub fn constraint(&mut self, constraint: JsonConstraint) -> &mut Self {
        match &constraint {
            JsonConstraint::ValueConstraint(data, order) => {
                assert_eq!(self.data_type, data.get_type());
                match self.data_type {
                    JsonType::Boolean => assert_eq!(order, &Ordering::Equal),
                    JsonType::Object => assert_eq!(order, &Ordering::Equal),

                    _ => {}
                }
            }
            _ => {}
        }
        self.constraints.insert(constraint);
        self
    }
}
impl ValidationJson {
    pub fn new() -> ValidationJson {
        ValidationJson { props: vec![] }
    }
    /// #  Description
    /// function adds property to the validation json
    /// # Arguments
    ///
    /// * `prop`: property to be added to the validation json
    ///
    /// returns: ValidationJson
    ///
    pub fn add(&mut self, prop: JsonValidationProperty) -> &mut Self {
        self.props.push(prop);
        self
    }
    pub fn get_all_props(&self) -> &Vec<JsonValidationProperty> {
        &self.props
    }
    pub fn get_all_unique_props(&self) -> Vec<&JsonValidationProperty> {
        self.props
            .iter()
            .filter(|&x| x.constraints.contains(&JsonConstraint::Unique))
            .collect::<Vec<&JsonValidationProperty>>()
    }
    fn comp_values_by_ordering<T: PartialEq + PartialOrd>(v1: T, v2: T, order: &Ordering) -> bool {
        match order {
            Ordering::Less => v1 < v2,
            Ordering::Equal => v1 == v2,
            Ordering::Greater => v1 > v2,
        }
    }

    fn comp_values(value1: &JsonData, value2: &JsonData, order: &Ordering) -> bool {
        match value1.get_type() {
            JsonType::String => {
                Self::comp_values_by_ordering(value1.as_string(), value2.as_string(), order)
            }
            JsonType::Integer => Self::comp_values_by_ordering(
                value1.as_int().unwrap(),
                value2.as_int().unwrap(),
                order,
            ),
            JsonType::Boolean => value1.as_bool().unwrap() == value2.as_bool().unwrap(),
            JsonType::Float => Self::comp_values_by_ordering(
                value1.as_float().unwrap(),
                value2.as_float().unwrap(),
                order,
            ),
            JsonType::Array => Self::comp_values_by_ordering(
                value1.as_array().unwrap(),
                value2.as_array().unwrap(),
                order,
            ),
            JsonType::Object => value1.as_object().unwrap() == value2.as_object().unwrap(),
            JsonType::Null => value2.is_null(),
        }
    }
    ///# Description
    /// function validates the json against the validation json
    /// # Arguments
    ///
    /// * `json`: json to be validated
    ///
    /// returns: Result<(), JsonValidationError>
    ///
    pub fn validate(&self, json: &JsonObject) -> Result<(), JsonValidationError> {
        for prop in self.props.iter() {
            if let Some(value) = json.get(&prop.name) {
                if (value.get_type() == prop.data_type)
                    || (prop.constraints.contains(&JsonConstraint::Nullable) && value.is_null())
                    || prop.constraints.contains(&JsonConstraint::Any)
                {
                    for constraint in prop.constraints.iter() {
                        match constraint {
                            JsonConstraint::Nullable => {
                                // all ready checked it in the if above
                            }
                            JsonConstraint::Any => {
                                // all ready checked it in the if above
                            }
                            JsonConstraint::Unique => {
                                //checked at the storage level
                            }
                            JsonConstraint::ValueConstraint(constraint_value, order) => {
                                if !Self::comp_values(value, constraint_value, order) {
                                    return Err(JsonValidationError::ValueDoesNotMeetConstraint(
                                        value.to_owned(),
                                        constraint_value.to_owned(),
                                        order.to_owned(),
                                    ));
                                }
                            }
                        }
                    }
                } else {
                    return Err(JsonValidationError::IncorrectType(
                        prop.name.to_owned(),
                        value.get_type(),
                        prop.data_type,
                    ));
                }
            } else {
                return Err(JsonValidationError::MissingProperty(prop.name.to_owned()));
            }
        }
        Ok(())
    }
}

impl TryFrom<JsonObject> for ValidationJson {
    type Error = JsonError;
    fn try_from(value: JsonObject) -> Result<Self, Self::Error> {
        let mut ret = Self::new();
        for (key, value) in value.into_iter() {
            let prop_as_json = value.as_object()?;
            let mut prop =
                JsonValidationProperty::new(key, prop_as_json["type"].to_owned().try_into()?);
            let constraints = prop_as_json["constraints"].as_object()?;
            for (key, value) in constraints.into_iter() {
                match key.as_str() {
                    "nullable" => {
                        prop.constraint(JsonConstraint::Nullable);
                    }
                    "any" => {
                        prop.constraint(JsonConstraint::Any);
                    }
                    "unique" => {
                        prop.constraint(JsonConstraint::Unique);
                    }
                    "value constraint" => {
                        let value = value.as_object()?;
                        let inner_value = value["value"].as_object()?;
                        let data_type=inner_value["type"].to_owned().try_into()?;
                        let data = JsonData::new(inner_value["data"].as_string(),data_type);
                        let order = value["order"].as_string();
                        let order = match order.as_str() {
                            "<" => Ordering::Less,
                            ">" => Ordering::Greater,
                            "=" => Ordering::Equal,
                            _ => return Err(JsonError::ParseError)?,
                        };
                        prop.constraint(JsonConstraint::ValueConstraint(data, order));
                    }
                    _ => {}
                }
            }
            ret.add(prop);
        }
        Ok(ret)
    }
}

impl Display for ValidationJson {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut self_as_json = JsonObject::new();
        for prop in &self.props {
            let mut prop_as_json = JsonObject::new();
            prop_as_json.insert(
                "type".to_string(),
                JsonData::from_string(prop.data_type.to_string()),
            );
            let mut constraints_as_json = JsonObject::new();
            for constraint in &prop.constraints {
                match constraint {
                    JsonConstraint::Nullable => {
                        constraints_as_json
                            .insert("nullable".to_string(), JsonData::from_boolean(true));
                    }
                    JsonConstraint::Any => {
                        constraints_as_json.insert("any".to_string(), JsonData::from_boolean(true));
                    }
                    JsonConstraint::Unique => {
                        constraints_as_json
                            .insert("unique".to_string(), JsonData::from_boolean(true));
                    }
                    JsonConstraint::ValueConstraint(constraint_value, order) => {
                        let mut constraint_as_json = JsonObject::new();
                        constraint_as_json.insert(
                            "value".to_string(),
                            JsonData::infer_from_string(constraint_value.to_string()).unwrap(),
                        );
                        constraint_as_json.insert(
                            "order".to_string(),
                            JsonData::from_string(
                                match order {
                                    Ordering::Less => "<",
                                    Ordering::Equal => "=",
                                    Ordering::Greater => ">",
                                }
                                .to_string(),
                            ),
                        );
                        constraints_as_json
                            .insert("value constraint".to_string(), constraint_as_json.into());
                    }
                }
            }
            prop_as_json.insert(
                "constraints".to_string(),
                JsonData::from(constraints_as_json),
            );
            self_as_json.insert(prop.name.clone(), JsonData::from(prop_as_json));
        }

        write!(f, "{}", JsonSerializer::serialize(self_as_json))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::JsonDeserializer;

    #[test]
    fn test_validate() {
        let mut gender_vp = JsonValidationProperty::new("gender".to_string(), JsonType::Boolean);
        let mut age_vp = JsonValidationProperty::new("age".to_string(), JsonType::Integer);
        age_vp
            .constraint(JsonConstraint::ValueConstraint(
                JsonData::from_int(15),
                Ordering::Greater,
            ))
            .constraint(JsonConstraint::Unique);
        gender_vp.constraint(JsonConstraint::Nullable);
        let mut template = ValidationJson::new();
        template
            .add(gender_vp)
            .add(JsonValidationProperty::new(
                "name".to_string(),
                JsonType::String,
            ))
            .add(age_vp);
        println!("{}", template);
        let mut json1 = JsonObject::new();
        json1.insert("gender".to_string(), JsonData::new_null());
        json1.insert(
            "name".to_string(),
            JsonData::from_string("adolf".to_string()),
        );
        json1.insert("age".to_string(), JsonData::from_int(17));

        assert!(template.validate(&json1).is_ok());

        let mut json2 = JsonObject::new();
        json2.insert("gender".to_string(), JsonData::new_null());
        json2.insert(
            "name".to_string(),
            JsonData::from_string("adolf".to_string()),
        );
        json2.insert("age".to_string(), JsonData::from_int(11));

        assert!(template.validate(&json2).is_err());

        let mut json3 = JsonObject::new();
        json3.insert("gender".to_string(), JsonData::from_boolean(false));
        json3.insert(
            "name".to_string(),
            JsonData::from_string("adolf".to_string()),
        );
        json3.insert("age".to_string(), JsonData::from_int(17));

        template.validate(&json3).expect("had an error");

        let mut json4 = JsonObject::new();
        json4.insert("gender".to_string(), JsonData::from_boolean(false));
        json4.insert("age".to_string(), JsonData::from_int(17));

        assert_eq!(
            template.validate(&json4).unwrap_err(),
            JsonValidationError::MissingProperty("name".to_string())
        );
    }
    #[test]
    fn test_from_json_object() {
        let json = r#"
        {
            "age": {
                    "constraints": {
                            "value constraint": {
                                    "order": "=",
                                    "value": {
                                            "data": 15,
                                            "type": "int"
                                    }
                            },
                            "unique": true
                    },
                    "type": "int"
            },
            "name": {
                    "constraints": {},
                    "type": "string"
            },
            "gender": {
                    "type": "bool",
                    "constraints": {
                            "nullable": true
                    }
            }
        }
        "#;
        let json = JsonDeserializer::deserialize(json.to_string()).unwrap();
        let template = ValidationJson::try_from(json).unwrap();
        assert_eq!(template.props.len(), 3);

        let prop = template.props.iter().find(|x| x.name == "name");
        assert_ne!(prop, None);
        let prop = prop.unwrap();
        assert_eq!(prop.data_type, JsonType::String);

        let prop = template.props.iter().find(|x| x.name == "gender");
        assert_ne!(prop, None);
        let prop = prop.unwrap();
        assert_eq!(prop.data_type, JsonType::Boolean);
        assert_eq!(prop.constraints.len(), 1);
        assert_ne!(prop.constraints.get(&JsonConstraint::Nullable), None);

        let prop = template.props.iter().find(|x| x.name == "age");
        assert_ne!(prop, None);
        let prop = prop.unwrap();
        assert_eq!(prop.data_type, JsonType::Integer);
        assert_eq!(prop.constraints.len(), 2);
        assert_ne!(
            prop.constraints.get(&JsonConstraint::ValueConstraint(
                JsonData::from_int(15),
                Ordering::Equal
            )),
            None
        );
        assert_ne!(prop.constraints.get(&JsonConstraint::Unique), None);
    }
}
