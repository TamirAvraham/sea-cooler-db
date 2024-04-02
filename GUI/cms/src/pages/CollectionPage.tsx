import React, {useEffect} from 'react';
import {useDispatch, useSelector} from "react-redux";
import {RootState} from "../reducers/store";
import RecordTag from "../components/RecordTag";
import ErrorComponent from "../components/ErrorComponent";
import {Loader} from "../components/Loader";
import {
    getCollectionRecords,
    resetError,
    resetGetCollectionRecordsStatus
} from "../reducers/CollectionsReducer";
import {UnknownAction} from "@reduxjs/toolkit";
import {useNavigate, useParams} from "react-router-dom";

export const CollectionPage = () => {
    const {collectionName}=useParams();
    const collections=useSelector((state:RootState)=>state.collection.collections);
    const collection=collections?.find(collection=>collection.name===collectionName);
    const userId=useSelector((state:RootState)=>state.user.user?.userId);
    const records=useSelector((state:RootState)=>state.collection.records)
    const recordsStatus=useSelector((state:RootState)=>state.collection.getCollectionRecordsStatus)
    const error=useSelector((state:RootState)=>state.collection.error)
    const dispatch=useDispatch();
    const navigate=useNavigate()


    useEffect(() => {
        dispatch(resetGetCollectionRecordsStatus())
        dispatch(resetError())
    }, [dispatch]);



    if (collection===undefined)
        return <ErrorComponent error={collectionName?`${collectionName} Was Not Found`:"No Connection Name"}/>
    const createRecord = () => {
        navigate('/record',{state:{record:undefined,collection:collection!}})
    }
    switch (recordsStatus) {
        case "idle":
            dispatch(getCollectionRecords({
                collection:collection,
                userId:userId!
            }) as unknown as UnknownAction)
            return <Loader/>
        case "loading":
            return <Loader/>
        case "error":
            return <ErrorComponent error={error}/>
        case "complete":
            return (
                <div className='collection-page'>
                    <h2 className="collection-name">{collectionName}</h2>
                    <p className="records">Records:</p>
                    <ul>
                        {(records)&&(records.map(record => <RecordTag record={record} collection={collection} key={record.name}/>))}
                    </ul>
                    <button className='create-record' onClick={createRecord}>Create Record</button>

                </div>
            );
    }

};

