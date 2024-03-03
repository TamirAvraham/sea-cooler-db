import {Collection, getCollectionsFromApi} from "../services/DatabaseInfoService";
import {AsyncStatus} from "./constents";
import {createAsyncThunk, createSlice} from "@reduxjs/toolkit";
import {createNewCollection} from "../services/CollectionService";
import {RootState} from "./store";

interface CollectionsState {
    collections?: Collection[];
    collectionsStatus:AsyncStatus
    createNewCollection:AsyncStatus
    error?:string
}
const defaultState:CollectionsState = {
    collections:undefined,
    collectionsStatus:'idle',
    createNewCollection:'idle',
    error:undefined,
}

export const getCollections=createAsyncThunk('collections/getCollections',async ()=>{
    console.log('started getting collections')
    return await getCollectionsFromApi()
})
export const createCollection = createAsyncThunk('collections/createCollection',async (params:{collection:Collection,userId:number},thunkAPI)=>{
    await createNewCollection(params.collection,params.userId)
    thunkAPI.dispatch(addCollection(params.collection))
})
const collectionsSlice=createSlice({
    name:'collections',
    initialState:defaultState,
    reducers:{
        addCollection:(state, action)=>{
            state.collections?.push(action.payload)
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
})
export default collectionsSlice.reducer
export const {addCollection} = collectionsSlice.actions