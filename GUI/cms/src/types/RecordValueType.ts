export type RecordValueType='int'|'bool'|'string'|'float'|'array'|'object'|'null';

export const validateRecordValue = (value:string,type:RecordValueType) => {
    switch (type) {
        case 'int':
            return validateInt(value);
        case 'float':
            return validateFloat(value);
        case 'array':
            return validateArray(value);
        case 'object':
            return validateObject(value);
        case 'bool':
            return validateBool(value);
        case 'string':
            return validateString(value);
        case "null":
            return value.toLowerCase() === 'null' || value==='';
        default:
            return false;
    }
}
const validateInt = (value:string) => {
    const num = parseFloat(value);
    return !isNaN(num) && Number.isInteger(num);
}
const validateFloat = (value:string) => {
    const num = parseFloat(value);
    return !isNaN(num);
}
const validateArray = (value:string) => {
    return value.startsWith('[') && value.endsWith(']');
}
const validateObject = (value:string) => {
    return value.startsWith('{') && value.endsWith('}');
}
const validateBool = (value:string) => {
    console.log(typeof value)
    return value.toLowerCase() === 'true' || value.toLowerCase() === 'false';
}
const validateString = (value:string) => {
    return value.length > 0;
}
export const ParseValue = (type:string,value:string):any => {
  switch (type.toLowerCase()) {
    case 'int':
      return parseInt(value);
    case 'float':
      return parseFloat(value);
    case 'bool':
      return value.toLowerCase() === 'true';
    case 'string':
      return value;
    case 'array':
      return JSON.parse(value);
    case 'object':
      return JSON.parse(value);
    default:
      return value;
  }
}