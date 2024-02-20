import {configureStore} from "@reduxjs/toolkit";
import UserReducer from "../reducers/UserReducer";

export const store=configureStore({
    reducer:{
        user:UserReducer
    }
})

export type RootState = ReturnType<typeof store.getState>;