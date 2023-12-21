use std::{cmp::Ordering, collections::HashSet};

use crate::json::{JsonData, JsonObject, JsonType};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonValidationError {
    IsNull,
    ValueDoesNotMeetConstraint(JsonData, JsonData, Ordering),
    MissingProperty,
    IncorrectType(String, JsonType, JsonType),
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonConstraint {
    Nullable,
    ValueConstraint(JsonData, Ordering),
    Unique,
    Any,
}

pub struct JsonValidationProperty {
    pub name: String,
    pub data_type: JsonType,
    pub constraints: HashSet<JsonConstraint>,
}
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
    pub fn constraint(mut self, constraint: JsonConstraint) -> Self {
        match &constraint {
            JsonConstraint::ValueConstraint(data, order) => {
                assert_eq!(self.data_type, data.get_type());
                match self.data_type {
                    JsonType::Boolean => assert_eq!(order, &Ordering::Equal),
                    JsonType::Object => assert_eq!(order, &Ordering::Equal),

                    _ => {}
                }
            }
            _=> {}
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
    pub fn add(mut self, prop: JsonValidationProperty) -> Self {
        self.props.push(prop);
        self
    }
    pub fn get_all_props(&self) -> &Vec<JsonValidationProperty>{
        &self.props
    }
    pub fn get_all_unique_props(&self) -> Vec<&JsonValidationProperty>{
        self.props.iter().filter(|&x| x.constraints.contains(&JsonConstraint::Unique)).collect::<Vec<&JsonValidationProperty>>()
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
                            },
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
                return Err(JsonValidationError::MissingProperty);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        let template = ValidationJson::new()
            .add(
                JsonValidationProperty::new("gender".to_string(), JsonType::Boolean)
                    .constraint(JsonConstraint::Nullable),
            )
            .add(JsonValidationProperty::new(
                "name".to_string(),
                JsonType::String,
            ))
            .add(
                JsonValidationProperty::new("age".to_string(), JsonType::Integer).constraint(
                    JsonConstraint::ValueConstraint(JsonData::from_int(15), Ordering::Greater),
                ),
            );

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
            JsonValidationError::MissingProperty
        );
    }
}
