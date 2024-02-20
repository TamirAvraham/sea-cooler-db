import React from 'react';
import logo from './logo.svg';
import './App.css';
import {Provider} from "react-redux";
import {store} from "./reducers/store";
import {UserLoginPage} from "./pages/UserLoginPage";
import {Header} from "./components/Header";

function App() {
  return (
    <Provider store={store}>
        <Header/>
      <UserLoginPage/>
    </Provider>
  );
}

export default App;
