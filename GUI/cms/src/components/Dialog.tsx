import React from "react";
import './Dialog.css'
export const Dialog = (params:{isOpen:boolean,onClose:()=>void,children:React.ReactNode}) =>
     params.isOpen ? <div className='overlay'>
            <div className='dialog'>
                {params.children}
            </div>
        </div>:null
