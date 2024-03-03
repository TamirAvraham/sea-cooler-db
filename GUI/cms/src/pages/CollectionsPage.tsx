import {useDispatch, useSelector} from "react-redux";
import {RootState} from "../reducers/store";
import {Loader} from "../components/Loader";
import {getCollections} from "../reducers/CollectionsReducer";
import {UnknownAction} from "@reduxjs/toolkit";
import {CollectionTag} from "../components/CollectionTag";
import {useEffect, useState} from "react";
import "./CollectionsPage.css";
import {Dialog} from "../components/Dialog";
import {CollectionCreator} from "../components/CollectionCreator";
import {useNavigate} from "react-router-dom";
const CollectionsView = () => {
    const collections = useSelector((state: RootState) => state.collection.collections);
    const navigator=useNavigate();
    return <div className="collections-page">
        <h2>Collections:</h2>
        <ul>
            {collections?.map((collection, index) =>
                <CollectionTag key={index} collection={collection} />) ?? <br />}
        </ul>
        <button onClick={event => navigator('/create_collection')}>Create Collection</button>
    </div>;
}
export const CollectionsPage = () => {
    const status = useSelector((state: RootState) => state.collection.collectionsStatus);
    const error = useSelector((state: RootState) => state.collection.error);
    const dispatch = useDispatch();

    useEffect(() => {
        if (status === "idle") {
            dispatch(getCollections() as unknown as UnknownAction);
        }

    }, [status, dispatch]);

    switch (status) {
        case "loading":
            return <Loader />;
        case "error":
            return <div className="error">
                <h2>Error: {error}</h2>
            </div>;
        case "complete":
            return <CollectionsView/>
        default:
            return <Loader />;
    }
};