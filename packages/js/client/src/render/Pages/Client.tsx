import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import styles from './Client.module.scss';
import UIStore, { ConnectionStatus } from '../store/Client/UIStore';
import Controls from '../components/Client/Controls';

const Client: React.FunctionComponent = observer(() => (
    <div className={styles.wrapper}>
        <div className={styles.frame}>
            <div className={styles.frameContent}>
                <Controls />
            </div>
        </div>
        {UIStore.connectionStatus === ConnectionStatus.Connected ? (
            <>
                {' '}
                <div className={styles.canvasWrapper}></div>
            </>
        ) : (
            <div className={styles.connectionStatusContainer}>
                <div className={styles.connectionStatus}>
                    <span>
                        {(() => {
                            switch (UIStore.connectionStatus) {
                                case ConnectionStatus.Connecting:
                                    return 'Connecting...';
                                case ConnectionStatus.Handshaking:
                                    return 'Handshaking...';
                                case ConnectionStatus.Disconnected:
                                    return 'Disconnected';
                                case ConnectionStatus.Error:
                                    return UIStore.error;
                                default:
                                    throw new Error(
                                        'Unknown connection status'
                                    );
                            }
                        })()}
                    </span>
                </div>
            </div>
        )}
    </div>
));

export default Client;
