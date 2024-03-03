import React, {useEffect, useState} from 'react';
import {uppercaseFirstLetterInString} from "../services/constents";
import {Collection, CollectionFiled} from "../services/DatabaseInfoService";
import {useDispatch, useSelector} from "react-redux";
import {useNavigate} from "react-router-dom";
import {createCollection} from "../reducers/CollectionsReducer";
import {RootState} from "../reducers/store";
import {UnknownAction} from "@reduxjs/toolkit";

interface CollectionField {
    name: string;
    type: 'string' | 'int' | 'float' | 'bool' | 'array' | 'object';
    constraints: Array<string>;
    valueConstraints: ValueConstraint[];
}

interface ValueConstraint {
    order: '<' | '=' | '>';
    value: string;
}
const isChecked=(constraint:string,field:CollectionField)=>field.constraints.includes(constraint)
const constraints=['any','unique','nullable'];
const formatValueConstraint=(valueConstraint:ValueConstraint,type:string)=>`${valueConstraint.order} ${valueConstraint.value} ${type}`
const transferPageDataToCollection=(collectionName:string, fields:CollectionField[])=>{
    return {
        name:collectionName,
        structure:fields.length===0?undefined:fields.map(filed=> {
                const ret: CollectionFiled = {
                    name: filed.name,
                    type: filed.type,
                    constraints: [...filed.constraints,...filed.valueConstraints.map(vc=>formatValueConstraint(vc,filed.type))]
                };
                return ret
            }
        )
    } as Collection
}
const CollectionCreationPage = () => {
    const [collectionName, setCollectionName] = useState('');
    const [fields, setFields] = useState<CollectionField[]>([]);
    const dispatch=useDispatch();
    const navigate=useNavigate();
    const userId=useSelector((state:RootState)=>state.user.user?.userId)
    const status=useSelector((state:RootState)=>state.collection.createNewCollection);
    useEffect(() => {
        if (userId===undefined) navigate('/')
    }, [navigate, userId]);
    useEffect(()=>{
        if (status==='complete') navigate('/collections')
    },[navigate,status]);
    const addField = () => {
        setFields([...fields, { name: '', type: 'string', constraints: [] ,valueConstraints:[]}]);
    };

    const updateFieldName = (index: number, name: string) => {
        const newFields = [...fields];
        newFields[index].name = name;
        setFields(newFields);
    };

    const updateFieldType = (index: number, type: CollectionField['type']) => {
        const newFields = [...fields];
        newFields[index].type = type;
        setFields(newFields);
    };

    const toggleConstraint = (index: number, constraint: string) => {
        const newFields = [...fields];
        if (newFields[index].constraints.includes(constraint)) {
            newFields[index].constraints=newFields[index].constraints.filter(value => value!==constraint)
        } else {
            newFields[index].constraints.push(constraint)
        }
        setFields(newFields);
    };
    const addValueConstraint = (index: number,constraint:ValueConstraint) => {
        const newFields = [...fields];
        newFields[index].valueConstraints.push(constraint);
        setFields(newFields);
    }
    const updateValueConstraintOrdering = (index: number, constraint:ValueConstraint['order'],i:number) => {
        const newFields = [...fields];
        newFields[index].valueConstraints[i].order = constraint;
        setFields(newFields);
    }
    const updateValueConstraintValue = (index: number, constraint:ValueConstraint['value'], i:number) => {
        const newFields = [...fields];
        newFields[index].valueConstraints[i].value = constraint;
        setFields(newFields);
    }

    const submit=()=>{
        dispatch(createCollection({collection:transferPageDataToCollection(collectionName,fields),userId:userId!}) as unknown as UnknownAction)
    }

    return (
        <div>
            <input
                type="text"
                value={collectionName}
                onChange={(e) => setCollectionName(e.target.value)}
                placeholder="Collection Name"
            />
            <button onClick={addField}>Add Field</button>
            {fields.map((field, index) => (
                <div key={index}>
                    <input
                        type="text"
                        value={field.name}
                        onChange={(e) => updateFieldName(index, e.target.value)}
                        placeholder="Field Name"
                    />
                    <select
                        value={field.type}
                        onChange={(e) => updateFieldType(index, e.target.value as CollectionField['type'])}
                    >
                        <option value="string">String</option>
                        <option value="int">Int</option>
                        <option value="float">Float</option>
                        <option value="bool">Bool</option>
                        <option value="array">Array</option>
                        <option value="object">Object</option>
                    </select>
                    {constraints.map(constraint=>
                        <label>
                            <input type="checkbox" name={constraint} id={constraint}
                            checked={isChecked(constraint,field)} onChange={()=>toggleConstraint(index,constraint)}/>
                            {uppercaseFirstLetterInString(constraint)}
                        </label>
                    )}
                    {(field.type!=='array' && field.type!=='object') && (
                        field.valueConstraints.map((value,i)=>
                            <div key={i}>
                            <select
                                value={value.order} onChange={(e)=>
                                updateValueConstraintOrdering(index, e.target.value as ValueConstraint['order'], i)}>

                                <option value="<">Less than</option>
                                <option value="=">Equal to</option>
                                <option value=">">Greater than</option>
                            </select>
                            <input type="text" value={value.value} onChange={(e)=>
                            updateValueConstraintValue(index, e.target.value, i)}/>
                            </div>
                        )
                    )}
                    {(field.type!=='array' && field.type!=='object') && (
                        <button onClick={e=> addValueConstraint(index,{
                            order:'<',
                            value:''
                        })}
                        > Add Value Constraint </button>
                    )}
                </div>
            ))}
            <button onClick={submit}>Create Collection</button>
        </div>
    );
};

export default CollectionCreationPage;