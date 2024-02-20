import React, {useRef} from "react";
import {useDispatch, useSelector} from "react-redux";
import {RootState} from "../reducers/store";
import {login} from "../reducers/UserReducer";
import {UnknownAction} from "@reduxjs/toolkit";
import {Loader} from "../components/Loader";
import './UserLoginPage.css'

const LoginForm = () => {
    const passwordReference=useRef<HTMLInputElement>(null)
    const usernameReference=useRef<HTMLInputElement>(null)
    const dispatch=useDispatch();
    const submitHandler = (e: any) => {
        e.preventDefault();
        const username = usernameReference.current?.value;
        const password = passwordReference.current?.value;
        if (username && password){
            dispatch(login({username, password}) as unknown as UnknownAction)
        }else{
            alert("Please enter a username and password")
        }
    }
  return (
      <form className='user-credentials-form' onSubmit={submitHandler}>
          <h2>Login</h2>
          <div className='form-input'>
              <label htmlFor='username'>Enter Your Username:</label>
              <input id='username' type={"text"} ref={usernameReference}/>
          </div>
          <div className='form-input'>
              <label htmlFor='password'>Enter Your Password:</label>
              <input id='password' type={"password"} ref={passwordReference}/>
          </div>
          <button className='form-button'>Login</button>
      </form>
  )
}

export const UserLoginPage = () => {
    const {status,error,user}=useSelector((state:RootState) => state.user);
    switch (status) {
        case 'loading':
            return <Loader/>
        case 'complete':
            return <div>Logged in id:{user!.userId}</div>
        case 'error':
            return <div>Error:{error!}</div>
        default:
            return <LoginForm/>
    }
}