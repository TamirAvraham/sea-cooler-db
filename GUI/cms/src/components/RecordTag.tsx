import React from 'react';
import {Record} from "../services/CollectionService";
import {Collection} from "../types/Collection";
import {useNavigate} from "react-router-dom";
const mapKnownFieldsIntoLi = (knownFields:Record['knownFields']) =>
    Object.entries(knownFields).map(([key,{value}])=>
        <li className='record-field'>
            <div className='record-known-field'>
                <h4>{key}</h4>
                <p>: {value}</p>
            </div>
        </li>
    )
const mapUnknownFieldsIntoLi = (unknownFields:Record['unknownFields']) => Object.entries(unknownFields).map(([key,value])=>
    <li className='record-field'>
        <div className='record-unknown-field'>
            <h4>{key}</h4>
            <p>: {value}</p>
        </div>
    </li>)

const RecordTag = (params:{record:Record,collection:Collection}) => {
    const navigate=useNavigate();
    return (
        <div className="record" onClick={()=>navigate('/record',{state:{record:params.record,collection:params.collection}})}>
            <h3 className='record-name'>{params.record.name}</h3>
            <ul className='record-known-fields'>{mapKnownFieldsIntoLi(params.record.knownFields)}</ul>
            <ul className='record-unknown-fields'>{mapUnknownFieldsIntoLi(params.record.unknownFields)}</ul>
        </div>
    );
};

export default RecordTag;