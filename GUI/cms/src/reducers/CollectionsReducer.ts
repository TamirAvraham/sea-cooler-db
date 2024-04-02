import {getCollectionsFromApi} from "../services/DatabaseInfoService";
import {AsyncStatus} from "./constents";
import {createAsyncThunk, createSlice, PayloadAction} from "@reduxjs/toolkit";
import {
    createNewCollection,
    getCollectionRecordsFromServer,
    Record,
    updateDocument
} from "../services/CollectionService";
import {RootState} from "./store";
import {Collection} from "../types/Collection";

interface CollectionsState {
    collections?: Collection[];
    collection?:Collection
    records?:Record[]

    collectionsStatus:AsyncStatus
    createNewCollection:AsyncStatus
    getCollectionRecordsStatus:AsyncStatus
    updateDocument:AsyncStatus,
    error?:string
}
const defaultState:CollectionsState = {
    collections:undefined,
    collectionsStatus:'idle',
    createNewCollection:'idle',
    getCollectionRecordsStatus:'idle',
    updateDocument:'idle',
    collection:undefined,
    error:undefined,
    records:undefined,
}
export const updateRecord=createAsyncThunk(
    'collections/updateRecord',
    async (params:{
        fields:{[p: string]: {type: string, value: string, nullable: boolean, any: boolean}},
        name:string, collectionName:string,
        extraFields:{[p: string]: string},
        userId:number
        },
    thunkAPI
    )=>{
    await updateDocument(params.extraFields, params.fields, params.name, params.collectionName, params.userId)
})
export const getCollections=createAsyncThunk('collections/getCollections',async ()=>{
    console.log('started getting collections')
    return await getCollectionsFromApi()
})
export const createCollection = createAsyncThunk('collections/createCollection',async (params:{collection:Collection,userId:number},thunkAPI)=>{
    await createNewCollection(params.collection,params.userId)
    thunkAPI.dispatch(addCollection(params.collection))
})
export const getCollectionRecords=createAsyncThunk(
    'collections/getCollectionRecords',
    async (params:{collection:Collection, userId:number}, thunkAPI)=>{
    return await getCollectionRecordsFromServer(params.userId,params.collection)
})
const collectionsSlice=createSlice({
    name:'collections',
    initialState:defaultState,
    reducers:{
        addCollection:(state, action)=>{
            state.collections?.push(action.payload)
        },
        getCollection(state,action:PayloadAction<string|undefined>){
            console.log(`collections ${state.collections}`)
            console.log(`collection ${state.collection}`)
            console.log(`action ${action.payload}`)
            if (action.payload === undefined || state.collections === undefined){
                state.collection=undefined;
                return
            }

            state.collection=state.collections?.find(collection=>collection.name===action.payload!)
        },
        resetGetCollectionRecordsStatus(state){
            state.getCollectionRecordsStatus='idle'

        },
        resetCreateNewCollectionStatus(state){
            state.createNewCollection='idle'
        },
        resetCollectionsStatus(state){
            state.collectionsStatus='idle'
        },
        resetError(state){
            state.error=undefined
        }
    },
    extraReducers:(builder)=>builder
        .addCase(getCollections.pending,(state)=>{
            state.collectionsStatus='loading'
        })
        .addCase(getCollections.fulfilled,(state, action)=>{
            state.collectionsStatus='complete'
            state.collections=action.payload
        })
        .addCase(getCollections.rejected,(state, action)=>{
            state.collectionsStatus='error'
            state.error=action.error.message
        }).addCase(
            createCollection.fulfilled,(state,action)=>{
                state.createNewCollection='complete'
                state.error=undefined
            }
        )
        .addCase(createCollection.rejected,(state, action)=>{
            state.createNewCollection='error'
            state.error=action.error.message
        })
        .addCase(createCollection.pending,(state)=>{
            state.createNewCollection='loading'
        })
        .addCase(getCollectionRecords.pending,(state)=>{
            state.getCollectionRecordsStatus='loading'
        })
        .addCase(getCollectionRecords.fulfilled,(state, action)=>{
            state.getCollectionRecordsStatus='complete'
            state.records=action.payload
        })
        .addCase(getCollectionRecords.rejected,(state, action)=>{
            state.getCollectionRecordsStatus='error'
            state.error=action.error.message
        })
        .addCase(updateRecord.pending,(state)=>{
            state.updateDocument='loading'
        })
        .addCase(updateRecord.fulfilled,(state)=>{
            state.updateDocument='complete'
        })
        .addCase(updateRecord.rejected,(state, action)=>{
            state.updateDocument='error'
            state.error=action.error.message
        })
})
export default collectionsSlice.reducer
export const {
    addCollection,
    getCollection,
    resetCreateNewCollectionStatus,
    resetGetCollectionRecordsStatus,
    resetCollectionsStatus,
    resetError,
} = collectionsSlice.actions