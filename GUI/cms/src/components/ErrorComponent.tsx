import React from 'react';

const ErrorComponent = (params:{error: string | undefined }) => {
    return (
        <div className='error'>
            Error: {params.error}
        </div>
    );
};

export default ErrorComponent;