use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum JsonType {
    String,
    Integer,
    Boolean,
    Float,
    Array,
    Object,
    Null,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JsonError {
    ParseError,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct JsonData {
    data: String,
    data_type: JsonType,
}

pub struct JsonDeserializer {}
pub struct JsonSerializer {}

pub type JsonArray = Vec<JsonData>;
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct JsonObject {
    map: HashMap<String, JsonData>,
}




impl JsonObject {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn insert(&mut self, k: String, v: JsonData) -> Option<JsonData> {
        self.map.insert(k, v)
    }
}
impl JsonType {
    pub fn get_type(data: &String) -> Result<JsonType, JsonError> {
        //null
        if data == "null" || data == "undefined" {
            return Ok(JsonType::Null);
        }
        //bool
        if data == "true" || data == "false" {
            return Ok(JsonType::Boolean);
        }

        //numbers
        if data.parse::<i128>().is_ok() {
            return Ok(JsonType::Integer);
        }
        if data.parse::<f64>().is_ok() {
            return Ok(JsonType::Float);
        }

        //collections
        match data.chars().nth(0) {
            Some(c) => match c {
                '[' => Ok(JsonType::Array),
                '\"' => Ok(JsonType::String),
                '{' => Ok(JsonType::Object),
                _ => Err(JsonError::ParseError),
            },
            None => Err(JsonError::ParseError),
        }
    }
}
impl JsonData {
    pub fn new(data: String, data_type: JsonType) -> JsonData {
        JsonData { data, data_type }
    }
    pub fn infer_from_string(data: String) -> Result<Self, JsonError> {
        let data_type = JsonType::get_type(&data)?;
        Ok(Self { data, data_type })
    }
    pub fn as_float(&self) -> Result<f32, JsonError> {
        self.try_into()
    }
    pub fn as_object(&self) -> Result<JsonObject, JsonError> {
        self.try_into()
    }
    pub fn as_array(&self) -> Result<JsonArray, JsonError> {
        self.try_into()
    }
    pub fn as_string(&self) -> String {
        self.into()
    }
    pub fn as_int(&self) -> Result<i32, JsonError> {
        self.try_into()
    }
    pub fn as_bool(&self) -> Result<bool, JsonError> {
        self.try_into()
    }
    pub fn is_null(&self) -> bool {
        self.data_type == JsonType::Null
    }
    pub fn from_string(v: String) -> JsonData {
        JsonData {
            data: format!("\"{}\"", v),
            data_type: JsonType::String,
        }
    }
    pub fn from_float(v: f32) -> JsonData {
        JsonData {
            data: v.to_string(),
            data_type: JsonType::Float,
        }
    }
    pub fn from_int(v: i32) -> JsonData {
        JsonData {
            data: v.to_string(),
            data_type: JsonType::Integer,
        }
    }
    pub fn from_boolean(v: bool) -> JsonData {
        JsonData {
            data: v.to_string(),
            data_type: JsonType::Boolean,
        }
    }
    pub fn new_null() -> JsonData {
        JsonData {
            data: "null".to_string(),
            data_type: JsonType::Null,
        }
    }
}

impl JsonDeserializer {
    fn get_key_position(data: &String) -> Result<(usize, usize), JsonError> {
        if let Some(key_start) = data.find("\"") {
            if let Some(key_end) = data[key_start + 1..].find("\"") {
                return Ok((key_start + 1, key_end));
            }
        }
        Err(JsonError::ParseError)
    }
    fn find_matching_closing_bracket(
        json_string: &str,
        start_index: usize,
    ) -> Result<usize, JsonError> {
        let mut bracket_count = 1;
        let open_bracket = json_string[start_index..]
            .chars()
            .next()
            .ok_or(JsonError::ParseError)?;
        let close_bracket = match open_bracket {
            '{' => '}',
            '[' => ']',
            _ => return Err(JsonError::ParseError),
        };

        for (i, c) in json_string[start_index + 1..].char_indices() {
            if c == open_bracket {
                bracket_count += 1;
            } else if c == close_bracket {
                bracket_count -= 1;
            }

            if bracket_count == 0 {
                return Ok(start_index + i + 1);
            }
        }

        Err(JsonError::ParseError)
    }
    fn get_last_value(data: &str, arr: bool) -> Result<usize, JsonError> {
        if data.find(if arr { '[' } else { '{' }).is_none() {
            if let Some(end) = data.find(if arr { ']' } else { '}' }) {
                return Ok(end);
            }
        }

        Err(JsonError::ParseError)
    }
    fn get_arr_member(data: &String) -> Result<(usize, usize), JsonError> {
        if let Some(first_char) = data[1..].chars().next() {
            let end = if first_char == '[' || first_char == '{' {
                Self::find_matching_closing_bracket(&data[1..], 0)? + 1
            } else {
                if let Some(end) = data[1..].find(",") {
                    end
                } else {
                    Self::get_last_value(&data[1..], true)?
                }
            };
            return Ok((1, end));
        }
        Err(JsonError::ParseError)
    }
    fn get_value_position(data: &String, arr: bool) -> Result<(usize, usize), JsonError> {
        if let Some(start) = data.find(':') {
            if let Some(first_char) = data[start + 1..].chars().next() {
                let end = if first_char == '[' || first_char == '{' {
                    Self::find_matching_closing_bracket(&data[start + 1..], 0)? + 1
                }
                //else if first_char == '\"'{
                //     data[start + 2..].find('\"').ok_or(JsonError::ParseError)?+1
                // }
                else {
                    if let Some(end) = data[start + 1..].find(",\"") {
                        end
                    } else {
                        Self::get_last_value(&data[start + 1..], arr)?
                    }
                };
                return Ok((start + 1, end));
            }
        }
        Err(JsonError::ParseError)
    }
    fn get_line(data: &String) -> Result<(String, String, usize), JsonError> {
        let (key_start, key_end) = Self::get_key_position(data)?;
        let (value_start, value_end) = Self::get_value_position(data, false)?;

        Ok((
            data[key_start..key_start + key_end].to_string(),
            data[value_start..value_start + value_end].to_string(),
            value_start + value_end,
        ))
    }
    fn clean_json(json: &String) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut escape = false;

        for c in json.chars() {
            if escape {
                // If the previous character was an escape character, don't check if it's the end of an escape sequence.
                result.push(c);
                escape = false;
            } else if c == '"' {
                // If it's a double quote, toggle in_string.
                result.push(c);
                in_string = !in_string;
            } else if in_string && c == '\\' {
                // If it's an escape character, handle the escape sequence.
                result.push(c);
                escape = true;
            } else if !in_string && c.is_whitespace() {
                // If it's outside of a string and is whitespace, skip it.
            } else {
                // Otherwise, add it to the result.
                result.push(c);
            }
        }

        result
    }
    pub fn deserialize_array(data: &String) -> Result<JsonArray, JsonError> {
        let mut data = data.clone();
        let mut ret = vec![];

        while data != "]" {
            let (value_start, value_end) = Self::get_arr_member(&data)?;

            let value = data[value_start..value_start + value_end].to_string();
            ret.push(JsonData::infer_from_string(value)?);

            data = data[value_end + 1..].to_string();
        }
        Ok(ret)
    }
    pub fn deserialize(mut data: String) -> Result<JsonObject, JsonError> {
        let mut ret = JsonObject::new();
        data = Self::clean_json(&data);

        while data != "" {
            let (key, value, pair_end) = Self::get_line(&data)?;
            ret.insert(key, JsonData::infer_from_string(value)?);

            data = data[pair_end + 1..].to_string();
        }
        Ok(ret)
    }
}

impl JsonSerializer {
    pub fn serialize(json: JsonObject) -> String {
        Self::serialize_with_spacer(json, 0, true)
    }
    pub fn serialize_with_spacer(json: JsonObject, spacer: u8, new_lines: bool) -> String {
        let mut ret = "{".to_string();
        if new_lines {
            ret.push('\n')
        }
        for (key, value) in json.into_iter() {
            if new_lines {
                for _ in 0..=spacer {
                    ret.push('\t')
                }
            }
            ret.push_str(&format!(
                "\"{}\": {},{}",
                key,
                value.to_json_string(spacer + 1, new_lines),
                if new_lines { "\n" } else { "" }
            ));
        }
        ret.pop();
        let c=ret.pop().unwrap();
        if !new_lines {
            ret.push(c)
        }
        if new_lines {
            ret.push('\n');
        }

        if new_lines {
            for _ in 1..spacer {
                ret.push('\t')
            }
        }
        ret.push('}');
        ret
    }
}

//basic collection impels for JsonObject

impl IntoIterator for JsonObject {
    type Item = (String, JsonData);

    type IntoIter = std::collections::hash_map::IntoIter<String, JsonData>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}
impl Index<String> for JsonObject {
    type Output = JsonData;

    fn index(&self, key: String) -> &Self::Output {
        self.map.get(&key).expect("Key not found")
    }
}

impl IndexMut<String> for JsonObject {
    fn index_mut(&mut self, key: String) -> &mut Self::Output {
        self.map.entry(key).or_insert(JsonData::new_null())
    }
}
impl Index<&str> for JsonObject {
    type Output = JsonData;

    fn index(&self, key: &str) -> &Self::Output {
        self.map.get(key).expect("Key not found")
    }
}

impl IndexMut<&str> for JsonObject {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.map
            .entry(key.to_string())
            .or_insert(JsonData::new_null())
    }
}

impl TryFrom<String> for JsonObject {
    type Error = JsonError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        JsonDeserializer::deserialize(value)
    }
}

impl JsonData {
    fn to_json_string(&self, spacer: u8, new_lines: bool) -> String {
        match self.data_type {
            JsonType::String => self.data.clone(),
            JsonType::Integer => self.data.clone(),
            JsonType::Boolean => self.data.clone(),
            JsonType::Float => self.data.clone(),
            JsonType::Array => {
                let mut ret = "[".to_string();
                for element in self.as_array().unwrap().into_iter() {
                    ret.push_str(&format!("{},", element.to_json_string(spacer, false)));
                }
                ret.pop();
                ret.push(']');
                ret
            }
            JsonType::Object => JsonSerializer::serialize_with_spacer(
                JsonObject::try_from(self.data.clone()).unwrap(),
                spacer + 1,
                new_lines,
            ),
            JsonType::Null => self.data.clone(),
        }
    }
}
//all into

impl Into<JsonData> for i32 {
    fn into(self) -> JsonData {
        JsonData::from_int(self)
    }
}

impl Into<JsonData> for f32 {
    fn into(self) -> JsonData {
        JsonData::from_float(self)
    }
}
impl Into<JsonData> for String {
    fn into(self) -> JsonData {
        JsonData::from_string(self)
    }
}
impl Into<JsonData> for bool {
    fn into(self) -> JsonData {
        JsonData::from_boolean(self)
    }
}

//all try from

impl TryFrom<JsonData> for i8 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i8>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for i16 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i16>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for i32 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for i64 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for i128 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i128>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for u8 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u8>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for u16 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u16>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for u32 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for u64 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for u128 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u128>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for f32 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Float {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<f32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<JsonData> for f64 {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Float {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<f64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}

impl TryFrom<JsonData> for bool {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Boolean {
            return Err(JsonError::ParseError);
        }
        Ok(value.data == "true")
    }
}
impl TryFrom<JsonData> for JsonArray {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Array {
            return Err(JsonError::ParseError);
        }
        JsonDeserializer::deserialize_array(&value.data)
    }
}
impl TryFrom<JsonData> for JsonObject {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Object {
            return Err(JsonError::ParseError);
        }
        JsonDeserializer::deserialize(value.data)
    }
}
impl From<JsonData> for String {
    fn from(item: JsonData) -> Self {
        item.data.replace("\"", "")
    }
}
impl TryFrom<&JsonData> for i8 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i8>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for i16 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i16>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for i32 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for i64 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for i128 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<i128>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for u8 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u8>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for u16 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u16>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for u32 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for u64 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for u128 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Integer {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<u128>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for f32 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Float {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<f32>()
            .map_err(|_| JsonError::ParseError)?)
    }
}
impl TryFrom<&JsonData> for f64 {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Float {
            return Err(JsonError::ParseError);
        }
        Ok(value
            .data
            .parse::<f64>()
            .map_err(|_| JsonError::ParseError)?)
    }
}

impl TryFrom<&JsonData> for bool {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Boolean {
            return Err(JsonError::ParseError);
        }
        Ok(value.data == "true")
    }
}
impl TryFrom<&JsonData> for JsonArray {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Array {
            return Err(JsonError::ParseError);
        }
        JsonDeserializer::deserialize_array(&value.data)
    }
}
impl TryFrom<&JsonData> for JsonObject {
    type Error = JsonError;

    fn try_from(value: &JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Object {
            return Err(JsonError::ParseError);
        }
        JsonDeserializer::deserialize(value.data.clone())
    }
}
impl From<&JsonData> for String {
    fn from(item: &JsonData) -> Self {
        item.data.replace("\"", "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_easy_json_decode() {
        let json_data = r#"
        {
            "name": "John",
            "age": 30,
            "city": "New York"
        }
        "#
        .to_string();

        let json = JsonDeserializer::deserialize(json_data);
        println!("{:?}", json);
        assert_ne!(json, Err(JsonError::ParseError));

        println!("json is: \n{}", JsonSerializer::serialize(json.unwrap()));
    }
    #[test]
    fn test_hard_json_decode() {
        let json_data = r#"
        {
            "string_key": "This is a string",
            "number_key": 42.5,
            "boolean_key": true,
            "null_key": null,
            "array_key": [1, 2, 3, "four", null, {"nested_key": "nested_value"}],
            "object_key": {
                "inner_string_key": "Hello, world!",
                "inner_number_key": 123,
                "inner_boolean_key": false,
                "inner_null_key": null,
                "inner_array_key": [true, false, "inner_string", {"deep_key": "deep_value"}]
            }
        }
        "#
        .to_string();

        let json = JsonDeserializer::deserialize(json_data);

        assert_ne!(json, Err(JsonError::ParseError));
        let json = json.unwrap();
        let _vector: JsonArray = (&json["array_key"]).try_into().unwrap();
        let float: f32 = (&json["number_key"]).try_into().unwrap();
        let object = json["object_key"].as_object().unwrap();
        let int = object["inner_number_key"].as_int().unwrap();
        let boolean = json["boolean_key"].as_bool().unwrap();
        let string = json["string_key"].as_string();
        let null = json["null_key"].is_null();

        assert_eq!(float, 42.5);
        assert_eq!(int, 123);
        assert_eq!(boolean, true);
        assert_eq!(string, "This is a string");
        assert!(null);

        println!("json is \n{}", JsonSerializer::serialize(json))
    }
}

//todo add labels
