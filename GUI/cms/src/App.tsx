import React from 'react';
import logo from './logo.svg';
import './App.css';
import {Provider} from "react-redux";
import {store} from "./reducers/store";
import {UserLoginPage} from "./pages/UserLoginPage";
import {Header} from "./components/Header";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import {CollectionsPage} from "./pages/CollectionsPage";
import {CollectionCreator} from "./components/CollectionCreator";
function App() {
  return (
    <Provider store={store}>
        <Router>
            <Header/>
            <Routes>
                <Route path="/" element={<UserLoginPage/>}/>
                <Route path="/collections" element={<CollectionsPage/>}/>
                <Route path="/create_collection" element={<CollectionCreator/>}/>
            </Routes>
        </Router>

    </Provider>
  );
}

export default App;
