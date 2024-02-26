import {configureStore} from "@reduxjs/toolkit";
import UserReducer from "../reducers/UserReducer";
import CollectionsReducer from "../reducers/CollectionsReducer";

export const store=configureStore({
    reducer:{
        user:UserReducer,
        collection:CollectionsReducer
    }
})

export type RootState = ReturnType<typeof store.getState>;