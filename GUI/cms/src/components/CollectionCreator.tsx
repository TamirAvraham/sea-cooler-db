import {useDispatch, useSelector} from "react-redux";
import {useEffect, useState} from "react";
import {uppercaseFirstLetterInString} from "../services/constents";
import {CollectionFiled} from "../services/DatabaseInfoService";
import {createCollection} from "../reducers/CollectionsReducer";
import {UnknownAction} from "@reduxjs/toolkit";
import {useNavigate} from "react-router-dom";
import {RootState} from "../reducers/store";
interface CollectionFiledConstraint {
  isChecked:boolean
  name:string
}

function formatConstraint(value: string,order:string) {
  return `${order} ${value}`;
}

const CollectionFiledCreator = (params:{filed:CollectionFiled,setFiled:(filed:CollectionFiled)=>void}) => {
  const [filedConstraints,setFiledConstraint]=useState<CollectionFiledConstraint[]>([
    {isChecked:false,name:'nullable'},
    {isChecked:false,name:'unique'},
    {isChecked:false,name:'any'},
    {isChecked:false,name:'value constraint'},
  ])
  const [name,setName]=useState<string>(params.filed.name)
  const [type,setType]=useState<string>(params.filed.type)
  const [value,setValue]=useState<string|undefined>(undefined)
  const [ordering,setOrdering]=useState<string|undefined>(undefined)
  const handleConstraintToggle = (name:string) => 
    setFiledConstraint(filedConstraints
        .map(option=> option.name===name ? {...option,isChecked:!option.isChecked} : option))

  useEffect(() => 
    params.setFiled({
      constraints: filedConstraints.filter(option=>option.isChecked).map(filed=>{
        if (filed.name==='value constraint' && value && ordering){
          return formatConstraint(value,ordering)
        } else {
          return filed.name
        }
      }),
      name: name,
      type: type
    })
  , [value, ordering, filedConstraints]);

  return <div className='collection-filed-creator'>
    <input type="text" name="name" id="name" className='filed-name'
           onChange={event=>setName(event.target.value)}/>
    <select name="type" id="type" className='filed-type'
            onChange={event => setType(event.target.value)}>
      <option value="string">String</option>
      <option value="int">Int</option>
      <option value="float">Float</option>
      <option value="bool">Bool</option>
      <option value="array">Array</option>
      <option value="object">Object</option>
    </select>
    <div className='constraints'>
    { //this is the constraint selector
      filedConstraints.map(option=> <div className="constraint-option">
      <label>
        <input type="checkbox"
               name={option.name}
               id={option.name}
               checked={option.isChecked}
               onChange={()=>handleConstraintToggle(option.name)}
        />
        {uppercaseFirstLetterInString(option.name)}
      </label>
    </div>)
    }
    {
      (filedConstraints.find(option=>option.name==='value constraint'&&option.isChecked))&&
        (
            <div className='value-constraint'>
              <input type="text" name="value" id="constraint-value" 
                     onChange={event=>setValue(event.target.value)}/>
              <select name="ordering" id="constraint-ordering" 
                      onChange={event => setOrdering(event.target.value)}>
                <option value="=">Equal (=)</option>
                <option value=">">Greater than (&gt;)</option>
                <option value="<">Less than (&lt;)</option>
              </select>
            </div>

        )
    }
    </div>
  </div>
}
const defaultFiled:CollectionFiled={
  name:'',
  type:'',
  constraints:[]
}
export const CollectionCreator = () => {
  const dispatch=useDispatch()
  const navigate=useNavigate()
  const [collectionName,setCollectionName]=useState<string|undefined>(undefined)
  const [collectionFields,setCollectionFields]=useState<CollectionFiled[]>([])
  const user=useSelector((state:RootState)=>state.user.user)
  const setFiled = (filed:CollectionFiled) => {
    collectionFields.map(f=> f.name===filed.name?filed:f)
  }
  const addFiled=()=>setCollectionFields([defaultFiled,...collectionFields])
  const handleSubmit = () => {
    if (!collectionName){
      return
    }
    dispatch(createCollection({collection:{
        name:collectionName,
        structure:collectionFields.length>0?collectionFields:undefined,
      },userId:user!.userId}) as unknown as UnknownAction)
    navigate('/collections')
  }
  return <div className='collection-creator' id='collection-creator'>
    <input type="text" placeholder="Collection Name" id="name" onChange={event=>setCollectionName(event.target.value)}/>
    <ul>
      {collectionFields.map((filed,index)=>
          <CollectionFiledCreator filed={filed} setFiled={setFiled} key={index}/>)}
    </ul>
    <button onClick={event => addFiled()}>Add Constraint</button>
    <button onClick={event => handleSubmit()}>CreateCollection</button>
  </div>
}