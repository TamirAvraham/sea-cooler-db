use crate::json::JsonError::ParseError;
use std::fmt::Display;
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

impl TryFrom<JsonData> for JsonType {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if let Self::String = value.data_type {
            match value.as_string().as_str() {
                "array" => Ok(Self::Array),
                "object" => Ok(Self::Object),
                "null" => Ok(Self::Null),
                "float" => Ok(Self::Float),
                "bool" => Ok(Self::Boolean),
                "string" => Ok(Self::String),
                "int" => Ok(Self::Integer),
                _ => return Err(JsonError::ParseError),
            }
        } else {
            return Err(JsonError::ParseError);
        }
    }
}

impl Display for JsonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            JsonType::String => "string",
            JsonType::Integer => "int",
            JsonType::Boolean => "bool",
            JsonType::Float => "float",
            JsonType::Array => "array",
            JsonType::Object => "object",
            JsonType::Null => "null",
        }
        .to_string();
        write!(f, "{}", str)
    }
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

impl Display for JsonData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut json = JsonObject::new();
        json.insert(
            "data".to_string(),
            JsonData::new(self.data.clone(), self.data_type),
        );
        json.insert(
            "type".to_string(),
            JsonData::from_string(self.data_type.to_string()),
        );
        write!(f, "{}", JsonSerializer::serialize(json))
    }
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
    /// # Description
    ///  the function inserts a json value with it's key if the key already exists it will replace the data and return the old data
    /// # Arguments
    ///
    /// * `k`: key to insert
    /// * `v`: data to insert
    ///
    /// returns: Option<JsonData> (it will return whether it replaced data and if it did it will return the old data)
    ///
    pub fn insert(&mut self, k: String, v: JsonData) -> Option<JsonData> {
        self.map.insert(k, v)
    }
    /// # Description
    ///  the function looks up a key in the json and returns its data (if it found it)
    /// # Arguments
    ///
    /// * `key`: key to look up in the object
    ///
    /// returns: Option<&JsonData>
    ///

    pub fn get(&self, key: &String) -> Option<&JsonData> {
        self.map.get(key)
    }
}
impl JsonType {
    /// # Description
    ///  function takes a string and returns its corresponding data type (if it found one)
    /// # Arguments
    ///
    /// * `data`: reference for string to check it's type
    ///
    /// returns: Result<JsonType, JsonError>
    ///
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
    /// # Description
    /// function tries to create an instance from a string and inferring it's type (will fail if cant detect type)
    /// # Arguments
    ///
    /// * `data`: data of the new instance
    ///
    /// returns: Result<JsonData, JsonError>
    pub fn infer_from_string(data: String) -> Result<Self, JsonError> {
        let data_type = JsonType::get_type(&data)?;
        Ok(Self { data, data_type })
    }
    //start of casters
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
    //end of casters\

    //start of froms
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
    //end of froms

    ///# Description
    /// function is a shortcut to creating a new null
    pub fn new_null() -> JsonData {
        JsonData {
            data: "null".to_string(),
            data_type: JsonType::Null,
        }
    }

    pub fn get_type(&self) -> JsonType {
        self.data_type
    }
}

impl JsonDeserializer {
    /// # Description
    ///  function checks for json keys in a string fails if it cant find any more keys
    /// # Arguments
    ///
    /// * `data`: reference to json string
    ///
    /// returns: Result<(usize, usize), JsonError> (key start,key end)
    ///
    fn get_key_position(data: &String) -> Result<(usize, usize), JsonError> {
        if let Some(key_start) = data.find("\"") {
            if let Some(key_end) = data[key_start + 1..].find("\"") {
                return Ok((key_start + 1, key_end));
            }
        }
        Err(JsonError::ParseError)
    }
    /// #  Description
    /// function looks for the matching closing bracket of a json object or array
    /// # Arguments
    ///
    /// * `json_string`: reference to json string
    /// * `start_index`: index to start looking for the closing bracket
    ///
    /// returns: Result<usize, JsonError> (index of the closing bracket)
    ///

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
    /// # Description
    ///  function tries to find the start and end of the last value in a json object string or a json array string depending on the arr flag
    /// # Arguments
    ///
    /// * `data`: cleaned json to look for the last value with in it
    /// * `arr`: if to look for an object ending(}) or look for an array ending(])
    ///
    /// returns: Result<usize, JsonError> (last value start)
    ///

    fn get_last_value(data: &str, arr: bool) -> Result<usize, JsonError> {
        if data.find(if arr { '[' } else { '{' }).is_none() {
            // look if there are different nested objects in the value
            if let Some(end) = data.find(if arr { ']' } else { '}' }) {
                return Ok(end);
            }
        }

        Err(JsonError::ParseError)
    }
    /// #  Description
    /// function tries to find the start and end of a value in a json array string
    /// # Arguments
    ///
    /// * `data`: reference to a json array string
    ///
    /// returns: Result<(usize, usize), JsonError> (member start, member end)
    ///

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
    /// #  Description
    /// function tries to find the start and end of a value in an json object string
    /// # Arguments
    ///
    /// * `data`: reference to a clean json
    ///
    /// returns: Result<(usize, usize), JsonError> (value start, value end)
    ///

    fn get_value_position(data: &String) -> Result<(usize, usize), JsonError> {
        if let Some(start) = data.find(':') {
            if let Some(first_char) = data[start + 1..].chars().next() {
                let end = if first_char == '[' || first_char == '{' {
                    Self::find_matching_closing_bracket(&data[start + 1..], 0)? + 1
                } else {
                    if let Some(end) = data[start + 1..].find(",\"") {
                        end
                    } else {
                        Self::get_last_value(&data[start + 1..], false)?
                    }
                };
                return Ok((start + 1, end));
            }
        }
        Err(JsonError::ParseError)
    }
    /// # Description
    ///  function selects a line from a cleaned json string
    /// # Arguments
    ///
    /// * `data`: a reference to cleaned json string
    ///
    /// returns: Result<(String, String, usize), JsonError>  (key, value, key value pair end)
    ///
    fn get_line(data: &String) -> Result<(String, String, usize), JsonError> {
        let (key_start, key_end) = Self::get_key_position(data)?;
        let (value_start, value_end) = Self::get_value_position(data)?;

        Ok((
            data[key_start..key_start + key_end].to_string(),
            data[value_start..value_start + value_end].to_string(),
            value_start + value_end,
        ))
    }

    /// # Description
    ///  function cleans up a json string so it can be parsed into a json object later by JsonDeserializer
    /// # Arguments
    ///
    /// * `json`: json string to clean
    ///
    /// returns: String
    ///

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
    /// # Description
    /// function deserializes a json string into a json array
    /// # Arguments
    ///
    /// * `data`: json array as string
    ///
    /// returns: Result<Vec<JsonData, Global>, JsonError>
    ///

    pub fn deserialize_array(data: &String) -> Result<JsonArray, JsonError> {
        if data == "[]" {
            return Ok(JsonArray::new());
        }
        let mut data = data.clone();
        let mut ret = JsonArray::new();

        while data != "]" {
            let (value_start, value_end) = Self::get_arr_member(&data)?;

            let value = data[value_start..value_start + value_end].to_string();
            ret.push(JsonData::infer_from_string(value)?);

            data = data[value_end + 1..].to_string();
        }
        Ok(ret)
    }

    /// # Description
    /// function deserializes a json string into a json object
    /// # Arguments
    ///
    /// * `data`: json object as string
    ///
    /// returns: Result<JsonObject, JsonError>
    ///

    pub fn deserialize(mut data: String) -> Result<JsonObject, JsonError> {
        if data == "" {
            return Err(ParseError);
        }
        let mut ret = JsonObject::new();
        data = Self::clean_json(&data);
        if data == "{}" {
            return Ok(ret);
        }
        while data != "" {
            let (key, value, pair_end) = Self::get_line(&data)?;
            ret.insert(key, JsonData::infer_from_string(value)?);

            data = data[pair_end + 1..].to_string();
        }
        Ok(ret)
    }
}

impl JsonSerializer {
    /// #  Description
    /// function serializes a json object into a string
    /// # Arguments
    ///
    /// * `json`: json object to serialize
    ///
    /// returns: String (json as string)
    ///
    pub fn serialize(json: JsonObject) -> String {
        Self::serialize_with_spacer(json, 0, true)
    }
    /// #  Description
    /// function serializes a json object into a string with a spacer
    /// # Arguments
    ///
    /// * `json`: json object to serialize
    /// * `spacer`: number of tabs
    /// * `new_lines`: add a new line at the end
    ///
    /// returns: String
    ///
    fn serialize_with_spacer(json: JsonObject, spacer: u8, new_lines: bool) -> String {
        if json.map.is_empty() {
            return "{}".to_string();
        }
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
        let c = ret.pop().unwrap();
        if !new_lines {
            ret.push(c)
        } else {
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
    pub fn serialize_array(json: JsonArray) -> String {
        if json.is_empty() {
            return "[]".to_string();
        }
        let mut ret = "[".to_string();
        for element in json.into_iter() {
            ret.push_str(&format!("{},", element.to_json_string(0, false)));
        }
        ret.pop();
        ret.push(']');
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
impl Index<&String> for JsonObject {
    type Output = JsonData;

    fn index(&self, key: &String) -> &Self::Output {
        self.map.get(key).expect("Key not found")
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
impl From<u128> for JsonData {
    fn from(value: u128) -> Self {
        JsonData {
            data_type: JsonType::Integer,
            data: value.to_string(),
        }
    }
}
impl JsonData {
    /// # Description
    /// function returns the json string version of the data
    /// # Arguments
    /// * `spacer`: number of tabs
    /// * `new_lines`: add a new line at the end
    /// # Returns
    ///  String (json string)
    pub fn to_json_string(&self, spacer: u8, new_lines: bool) -> String {
        match self.data_type {
            JsonType::String => self.data.clone(),
            JsonType::Integer => self.data.clone(),
            JsonType::Boolean => self.data.clone(),
            JsonType::Float => self.data.clone(),
            JsonType::Array => JsonSerializer::serialize_array(self.as_array().unwrap()),
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
impl Into<JsonData> for u8 {
    fn into(self) -> JsonData {
        JsonData::from_int(self as i32)
    }
}
impl Into<JsonData> for u16 {
    fn into(self) -> JsonData {
        JsonData::from_int(self as i32)
    }
}

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
impl From<JsonObject> for JsonData {
    fn from(value: JsonObject) -> Self {
        Self::infer_from_string(JsonSerializer::serialize(value)).unwrap()
    }
}
impl<T> From<Vec<T>> for JsonData
where
    T: Into<JsonData>,
{
    fn from(vec: Vec<T>) -> Self {
        let mut array = JsonArray::new();
        for item in vec {
            array.push(item.into());
        }
        JsonData::new(JsonSerializer::serialize_array(array), JsonType::Array)
    }
}

impl<T> TryFrom<JsonData> for Vec<T>
where
    T: TryFrom<JsonData>,
{
    type Error = JsonError;
    fn try_from(json: JsonData) -> Result<Self, Self::Error> {
        let mut vec = Vec::new();
        if json.data_type == JsonType::Array {
            for item in JsonDeserializer::deserialize_array(&json.data)? {
                vec.push(T::try_from(item).map_err(|_| JsonError::ParseError)?);
            }
        }
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create_json() {
        let mut json = JsonObject::new();
        println!("json is: \n{}", JsonSerializer::serialize(json.clone()));
        json.insert("name".to_string(), "John".to_string().into());
        json.insert("age".to_string(), 30.into());
        json.insert("city".to_string(), "New York".to_string().into());
        println!("json is: \n{}", JsonSerializer::serialize(json.clone()));
    }
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
    #[test]
    fn test_array_test() {
        let mut array = vec![1u8, 2, 3, 4, 5];
        let mut json: JsonData = array.try_into().unwrap();
        println!("json is \n{}", json.as_string());
    }
    #[test]
    fn test_single_json() {
        let mut json = JsonObject::new();
        json.insert(
            "test".to_string(),
            JsonData::from_string("test".to_string()),
        );
        println!("json is: \n{}", JsonSerializer::serialize(json.clone()));
    }
    #[test]
    fn test_empty_array() {
        let mut json = JsonObject::new();
        let json_arr = JsonArray::new();
        let json_arr_internal: JsonData = json_arr.into();
        println!("json is: \n{}", json_arr_internal.data);
        json.insert("test".to_string(), json_arr_internal);

        println!("json is: \n{}", JsonSerializer::serialize(json));
    }
}
