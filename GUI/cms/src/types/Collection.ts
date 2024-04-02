export interface CollectionFiled{
    name:string
    type:string,
    constraints:string[]
}
export interface Collection{
    name:string
    structure?:CollectionFiled[]
}