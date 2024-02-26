import {uppercaseFirstLetterInString, URL} from "./constents";

export interface CollectionFiled{
    name:string
    type:string,
    constraints:string[]
}
export interface Collection{
    name:string
    structure?:CollectionFiled[]
}
function parseCollectionFromJsonKeyValuePair(key:string,value:any):Collection {
    if (value["structure"]===null){
        return {name:key, structure:undefined} as Collection
    }else{
        const fields=Object.entries(value["structure"]).map(([key, v])=>{

            const value=v as any;
            const constraints:string[]=Object.entries(value["constraints"]).map(([key,v])=>{
                console.log(`name:${key}`)
                if (key==="value constraint"){
                    const value=v as any;
                    return ` ${value["order"]} ${value["value"]["data"]} ${uppercaseFirstLetterInString(value["value"]["type"])}`;
                } else {
                    return uppercaseFirstLetterInString(key)
                }
            })
            return {name:key, type:uppercaseFirstLetterInString(value["type"]), constraints:constraints} as CollectionFiled;
        });
        return {name:key, structure:fields} as Collection
    }
}
function parseCollectionsJson(json:any):Collection[]{
    console.log(json)
    return ((json["collections"] ?? []) as Array<any>).map((collectionObject)=>{
        const [key,value]=Object.entries(collectionObject)[0];
        return parseCollectionFromJsonKeyValuePair(key, value as any) as Collection;
    })
}
export const getCollectionsFromApi = async () =>
    await fetch(`${URL}/collections`)
      .then((res)=>res.json())
      .then((json)=>parseCollectionsJson(json))


