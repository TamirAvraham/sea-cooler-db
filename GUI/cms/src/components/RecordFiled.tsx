import React, {useEffect, useState} from 'react';
import {CollectionFiled} from "../types/Collection";
import {RecordValueType, validateRecordValue} from "../types/RecordValueType";
import {uppercaseFirstLetterInString} from "../services/constents";
export type RecordFiledProps = {name:string,type:RecordValueType,value?:string,onChange:(value:string)=>void,nullable:boolean,any:boolean}
export const RecordFiled = (params:RecordFiledProps) => {
    console.log(params)
    const type=uppercaseFirstLetterInString(params.type)
    return (
        <div className='record-filed'>
            <h3>{params.name}</h3>
            <p>:{type}</p>
            <input
                type="text"
                name="value"
                id="value"
                onChange={e=>params.onChange(e.target.value)} value={params.value || ''}
            />
            {!params.any &&
                (
                    !validateRecordValue(params.value??'', params.type)
                    ||
                    (params.nullable && !validateRecordValue(params.value??'', 'null'))
                ) && (<p className="error">Value Is Not {type}</p>)
            }
        </div>
    );
};

export const ExtraFiled = (params:{onChange:(name:string,value:string)=>void,name:string,value:string}) => {
    const [name, setName] = useState(params.name)
    const [value, setValue] = useState(params.value)
    useEffect(() => {
        params.onChange(name, value)
    }, [name,value]);
  return (
      <div className="extra-record-filed">
          <label className='value'>
              <input
                  type="text"
                  name="name"
                  id="name"
                  onChange={e=>setName(e.target.value)}
                />
                Name:
          </label>
          <label className='value'>
              <input
                  type="text"
                  name="value"
                  id="value"
                  onChange={e=>setValue(e.target.value)}
              />
              Value:
          </label>
      </div>
  )
}

