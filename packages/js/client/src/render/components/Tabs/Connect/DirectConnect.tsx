import React, { useEffect, useState } from 'react';
import styles from './ConnectToOthers.module.scss';
import Title from './Title';
import Input from '../../Utility/Input';
import formatID from '../../../helper/formatID';

const DirectConnect: React.FunctionComponent = () => {
    const [connectID, setConnectID] = useState('');

    useEffect(() => {
        // TODO cursor moves to end
        setConnectID(formatID(connectID));
    }, [connectID]);

    return (
        <>
            <Title>Connect To Remote Computer</Title>
            <label>
                <div className={styles.labelText}>Partner ID</div>
                <Input
                    value={connectID}
                    onChange={(e) => setConnectID(e.target.value)}
                    className={styles.input}
                />
            </label>
            <div className={styles.options}>
                <label>
                    <input
                        type={'radio'}
                        name={'directConnect.connectionType'}
                        defaultChecked={true}
                    />
                    <span className={styles.optionText}>View or Control</span>
                </label>
                <label>
                    <input
                        type={'radio'}
                        name={'directConnect.connectionType'}
                    />
                    <span className={styles.optionText}>File Transfer</span>
                </label>
            </div>
        </>
    );
};
export default DirectConnect;
