import { createAsyncThunk, createSlice } from "@reduxjs/toolkit";
import { User, UserService } from "../services/UserService";
import { AsyncStatus } from "./constents";
interface UserState{
    user?:User
    status:AsyncStatus
    error?:string

}
const defaultState:UserState={
    user:undefined,
    status: 'idle',
    error:undefined
}
export const login=createAsyncThunk('user/login',async (params:{username:string,password:string}) => {
    return await UserService.login(params.username,params.password)
})
export const signup=createAsyncThunk('user/signup',async (params:{username:string,password:string,permissions:any}) => {
    return await UserService.signup(params.username,params.password,params.permissions)
})
export const logout=createAsyncThunk('user/logout',async (params:{userId?:number},thunkApi) => {
    let state=thunkApi.getState() as UserState;
    if (params.userId===undefined && state.user===undefined) {
        throw new Error("Improper parameters from logout");
    }
    return await UserService.logout(params.userId??state.user!.userId)
})
export const userSlice=createSlice({
    name:'user',
    initialState:defaultState,
    reducers:{},
    extraReducers:(builder)=>builder
        .addCase(login.pending,(state)=>{
            state.status='loading'
        }).addCase(login.fulfilled,(state,action)=>{
            state.status='complete'
            state.user=action.payload
        }).addCase(login.rejected,(state, action)=>{
            state.status='error'
            state.error=action.error.message
        }).addCase(signup.pending,(state)=>{
                state.status='loading'
        }).addCase(signup.fulfilled,(state, action)=>{
            state.status='complete'
            state.user=action.payload
        }).addCase(signup.rejected,(state, action)=>{
            state.status='error'
            state.error=action.error.message
        }).addCase(logout.pending,(state)=>{
            state.status='loading'
        }).addCase(logout.fulfilled,(state)=>{
            state.status='complete'
            state.user=undefined
        }).addCase(logout.rejected,(state, action)=>{
            state.status='error'
            state.error=action.error.message
        })
})
export default userSlice.reducer;