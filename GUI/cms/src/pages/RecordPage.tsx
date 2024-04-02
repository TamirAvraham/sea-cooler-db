import React, {useState} from 'react';
import {Record} from "../services/CollectionService";
import {Collection} from "../types/Collection";
import {ExtraFiled, RecordFiled, RecordFiledProps} from "../components/RecordFiled";
import {useLocation} from "react-router-dom";
const createFieldsFromCollection = (collection:Collection) => {
    const ret:Record['knownFields']={}
    collection.structure?.forEach(field=>{
        ret[field.name]= {
            type:field.type,
            value:'',
            nullable:field.constraints.includes('Nullable'),
            any:field.constraints.includes('Any'),
        }
    })
    return ret
}
const RecordPage = () => {
    const {record,collection}=useLocation().state as {record?: Record, collection:Collection}
    const [name, setName] = useState(record?.name)
    const [fields, setFields] = useState(record?.knownFields
        ?? createFieldsFromCollection(collection))
    const [extraFields, setExtraFields] = useState(record?.unknownFields ?? {})
    const onFiledValueChange = (fieldName:string) => {
        return (value:string)=>{
            setFields({...fields, [fieldName]:{...fields[fieldName], value}})
        }
    }
    const onExtraFieldChange = (index:number) =>  {
      return (name:string,value:string)=>{
          const newExtraFields = Object.entries(extraFields).map(([key,value], i)=>{
              if(i===index) return [name, value]
              return [key, value]
          })
          setExtraFields(Object.fromEntries(newExtraFields))
      }
    }

    console.log(record)
    return (
        <div className='record-page'>
            <label className="name">
                <input
                    type="text"
                    name="name"
                    id="name"
                    value={name??''}
                    onChange={e=>setName(e.target.value)}
                />
                Name:
            </label>
            <div className="fields">
                {Object.entries(fields).map(([key,value])=>
                    <RecordFiled
                        name={key}
                        type={value.type.toLocaleLowerCase() as RecordFiledProps['type']}
                        value={`${value.value}`}
                        nullable={value.nullable}
                        any={value.any}
                        onChange={onFiledValueChange(key)}
                        key={key}
                    />
                )}
            </div>
            <div className='extra-fields'>
                {Object.entries(extraFields).map(([key, value],index)=>
                    <ExtraFiled onChange={onExtraFieldChange(index)} name={key} value={value}/>
                )}
            </div>
            <button
                onClick={e=>setExtraFields({...extraFields, new:''})}
                className='add-extra-field'>
                Add Extra Field
            </button>

        </div>
    );
};

export default RecordPage;