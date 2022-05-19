import { ipcRenderer } from 'electron';
import React, { useEffect, useState } from 'react';
import styles from './ConnectToOthers.module.scss';
import Title from './Title';
import Input from '../../../Utility/Input';
import formatID from '../../../../helper/formatID';
import Button from '../../../Utility/Button';
import { RendererToMainIPCEvents } from '../../../../../common/IPCEvents';

const ConnectToOthers: React.FunctionComponent = () => {
    const [connectID, setConnectID] = useState('');

    useEffect(() => {
        // TODO cursor moves to end
        setConnectID(formatID(connectID));
    }, [connectID]);

    return (
        <>
            <Title>Connect To Remote Computer</Title>
            <label>
                <div className={styles.labelText}>Partner ID or Server</div>
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
                        disabled={true}
                        name={'directConnect.connectionType'}
                    />
                    <span className={styles.optionText}>File Transfer</span>
                </label>
                <div className={styles.button}>
                    <Button
                        onClick={() => {
                            ipcRenderer.send(
                                RendererToMainIPCEvents.EstablishSession,
                                connectID
                            );
                            setConnectID('');
                        }}
                    >
                        Connect
                    </Button>
                </div>
            </div>
        </>
    );
};
export default ConnectToOthers;
