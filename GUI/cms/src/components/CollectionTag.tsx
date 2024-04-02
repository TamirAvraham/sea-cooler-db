import {Collection, CollectionFiled} from "../types/Collection";
import React from "react";
import "./CollectionTag.css"
import {useNavigate} from "react-router-dom";
const CollectionFiledComponent = (props:{filed: CollectionFiled}) => {
  return <div className="collection-filed">
      <h4>{props.filed.name}: </h4>
      <p>{props.filed.constraints.find((constraint)=>constraint==="any")===undefined?props.filed.type:"Any"}</p>
      <ul>{props.filed.constraints.map((constraint)=><li>{constraint}</li>)}</ul>
  </div>
}
export const CollectionTag = (props:{collection:Collection}) => {
    const navigate=useNavigate();
    const onClick=()=>navigate(`/collection/${props.collection.name}`)
    const bottomValue= props.collection.structure
            ?.map((filed)=><CollectionFiledComponent filed={filed}/>)
        ?? <p>None</p>
  return (
      <div className="collection-tag card" onClick={onClick}>
          <h3>{props.collection.name}</h3>
          {bottomValue}
      </div>
  )
}