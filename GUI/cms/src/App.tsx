import React from 'react';

import './App.css';
import {Provider} from "react-redux";
import {store} from "./reducers/store";
import {UserLoginPage} from "./pages/UserLoginPage";
import {Header} from "./components/Header";
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import {CollectionsPage} from "./pages/CollectionsPage";
import CollectionCreationPage from "./pages/CollectionCreationPage";
import {CollectionPage} from "./pages/CollectionPage";
import RecordPage from "./pages/RecordPage";
function App() {
  return (
    <Provider store={store}>
        <Router>
            <Header/>
            <Routes>
                <Route path="/" element={<UserLoginPage/>}/>
                <Route path="/collections" element={<CollectionsPage/>}/>
                <Route path="/create_collection" element={<CollectionCreationPage/>}/>
                <Route path="/collection/:collectionName" element={<CollectionPage/>}/>
                <Route path="record" element={<RecordPage/>}/>
            </Routes>
        </Router>

    </Provider>
  );
}

export default App;
