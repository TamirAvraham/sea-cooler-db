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
pub enum JsonError {
    ParseError,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JsonData {
    data: String,
    data_type: JsonType,
}
impl JsonType {
    pub fn get_type(data: &String) -> Result<JsonType, JsonError> {
        if data == "null" || data == "undefined" {
            return Ok(JsonType::Null);
        }

        if data.parse::<f64>().is_ok() {
            return Ok(JsonType::Float);
        }

        if data.parse::<i128>().is_ok() {
            return Ok(JsonType::Integer);
        }
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
    pub fn from_string(data: String)->Result<Self,JsonError>{
        let data_type = JsonType::get_type(&data)?;
        Ok(Self{ data, data_type })
    }
}
