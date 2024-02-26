import {Collection} from "./DatabaseInfoService";
import {URL} from "./constents";

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
const fromatCollectionAsJson=(collection:Collection)=>{
    let structureObject={}
    if (collection.structure){
        for (const collectionElement of collection.structure!) {
            // @ts-ignore
            structureObject[collectionElement.name]= {
                "type": collectionElement.type,
                "constraint": collectionElement.constraints.map(constraint=>formatConstraint(constraint))
                    .reduce((acc,[key,value])=>{acc[key] = value; return acc;}, {} as any),
            }
        }
    }
    
    return {
        "collection_structure":structureObject,
        "collection_name":collection.name,
    }
}
export const createNewCollection=async (collection:Collection,userId:number)=>{
    await fetch(`${URL}/create_new_collection?${userId}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        }
        ,body:JSON.stringify(fromatCollectionAsJson(collection))
    })
}