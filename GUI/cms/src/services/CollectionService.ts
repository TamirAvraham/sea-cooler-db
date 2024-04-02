
import {URL} from "./constents";
import {Collection} from "../types/Collection";
import {ParseValue} from "../types/RecordValueType";

const formatValueConstraint=(constraint:string)=>{
    const [order,value,type]=constraint.split(' ')
    return {
        "order": order,
        "value": {
            "data":value,
            "type": type,
        }
    }

}
const formatConstraint=(constraint:string):[string,any]=>{
    if (constraint[0]==='='|| constraint[0]==='>' || constraint[0]==='<'){
        return ["value constraint",formatValueConstraint(constraint)]
    }else{
        return [constraint.toLowerCase(),true]
    }
}
const formatCollectionAsJson=(collection:Collection)=>{
    let structureObject={}
    if (collection.structure){
        for (const collectionElement of collection.structure!) {
            // @ts-ignore
            structureObject[collectionElement.name]= {
                "type": collectionElement.type,
                "constraints": collectionElement.constraints.map(constraint=>formatConstraint(constraint))
                    .reduce((acc,[key,value])=>{acc[key] = value; return acc;}, {} as any),
            }
        }
    }
    
    const ret= {
        "collection_structure":structureObject,
        "collection_name":collection.name,
    }
    console.log(ret)
    return ret
}
export const createNewCollection=async (collection:Collection,userId:number)=>{
    await fetch(`${URL}/create_new_collection?user_id=${userId}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        }
        ,body:JSON.stringify(formatCollectionAsJson(collection))
    })
}
export interface Record {
   knownFields: {[key:string]:{type:string,value:string,nullable:boolean,any:boolean}}
    unknownFields: {[key:string]:string}
    name:string
}
export const getCollectionRecordsFromServer=async (userId:number,collection:Collection):Promise<Record[]>=>{
    const data=(await fetch(`${URL}/collection?user_id=${userId}&collection_name=${collection.name}`)
        .then(res=>res.json()))["documents"] as any[]
    return data.map(recordObject => {
        const fields:Record["knownFields"]={}
        const unknownFields:Record["unknownFields"]={}
        Object.entries(recordObject["data"]).map(([key,value]):[string,any,string|undefined,boolean|undefined,boolean|undefined]=>{
            const filed=collection.structure?.find(filed => filed.name === key)
            return [key,value,filed?.type,filed?.constraints.includes('Nullable'),filed?.constraints.includes('Any')]
        }

        ).forEach(([key,value,type,nullable,any])=>type?fields[key]={type:type!,value,nullable:nullable??false,any:any??false}:unknownFields[key]=value)
        return {
            name:recordObject["document_name"], knownFields:fields, unknownFields:unknownFields
        } as Record
    })
}
export const updateDocument =async (
    extraFields: {[p: string]: string},
    fields:{[p: string]: {type: string, value: string, nullable: boolean, any: boolean}},
    name:string,
    collectionName:string,
    userId:number,
) => {
  await fetch(`${URL}/collection?user_id=${userId}&collection_name=${collectionName}`, {
      headers: {'Content-Type': 'application/json'},
      method: 'PUT',
      body: JSON.stringify({
          document_name: name,
          data: Object.entries(fields).reduce((acc, [key, {value, type}]) => {
              acc[key] = ParseValue(type, value)
              return acc
          }, {...extraFields})
      })
  })
}