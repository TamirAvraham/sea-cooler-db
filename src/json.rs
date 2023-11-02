use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonData {
    data: String,
    data_type: JsonType,
}

pub struct JsonDeserializer {}

pub type JsonObject = HashMap<String, JsonData>;
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
    pub fn from_string(data: String) -> Result<Self, JsonError> {
        let data_type = JsonType::get_type(&data)?;
        Ok(Self { data, data_type })
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
    fn find_scope_end(c: char, data: &str) -> usize {
        todo!()
    }
    fn get_last_value(data: &str)->Result<usize,JsonError>{
        if data.find('{').is_none() {
            if let Some(end) = data.find('}') {
                return Ok(end);
            }
        }

        Err(JsonError::ParseError)
    }
    fn get_value_position(data: &String) -> Result<(usize, usize), JsonError> {
        if let Some(start) = data.find(":") {
            if let Some(first_char) = data[start + 1..].chars().next() {
                let end = if first_char == '[' || first_char == '{' {
                    Self::find_scope_end(first_char, &data[start + 1..])
                 } //else if first_char == '\"'{
                //     data[start + 2..].find('\"').ok_or(JsonError::ParseError)?+1
                // } 
                else {
                    
                    if let Some(end) = data[start+1..].find(",") {
                        end
                    } else {
                        Self::get_last_value(&data[start+1..])?
                    }
                };
                println!("first char is {}", first_char);
                return Ok((start + 1, end));
            }
        }
        Err(JsonError::ParseError)
    }
    fn get_line(data: &String) -> Result<(String, String, usize), JsonError> {
        let (key_start, key_end) = Self::get_key_position(data)?;
        let (value_start, value_end) = Self::get_value_position(data)?;

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
    pub fn deserialize_array(data: String) -> Result<Vec<JsonData>, JsonError> {
        todo!()
    }
    pub fn deserialize(mut data: String) -> Result<JsonObject, JsonError> {
        let mut ret = HashMap::new();
        data = Self::clean_json(&data);

        while data != "" {
            println!("started parsing new line. data is {}.", data);
            let (key, value, pair_end) = Self::get_line(&data)?;
            println!(
                "got new key value pair. key={},value={},pair_end={}",
                key, value, pair_end
            );
            ret.insert(key, JsonData::from_string(value)?);

            data = data[pair_end+1..].to_string();
        }
        Ok(ret)
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

impl TryFrom<JsonData> for Vec<JsonData> {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Array {
            return Err(JsonError::ParseError);
        }
        JsonDeserializer::deserialize_array(value.data)
    }
}
impl TryFrom<JsonData> for JsonObject {
    type Error = JsonError;

    fn try_from(value: JsonData) -> Result<Self, Self::Error> {
        if value.data_type != JsonType::Array {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_json_decode() {
        let json_data = r#"
        {
            "name": "John",
            "age": 30,
            "city": "New York"
        }
        "#
        .to_string();
        let json = JsonDeserializer::deserialize(json_data);
        print!("{:?}",json);
        assert_ne!(json, Err(JsonError::ParseError))
    }
}

//todo add labels
