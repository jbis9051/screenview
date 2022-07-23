import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import { ipcRenderer } from 'electron';
import styles from './Host.module.scss';
import UIStore from '../store/Host/UIStore';
import DisplaySelector from '../components/Host/DisplaySelector';
import startDesktopSelection from '../helper/Host/startDesktopSelection';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';
import { HostHeight } from '../../common/contants';

const Host = observer(() => (
    <div className={styles.wrapper}>
        {UIStore.inSelectionMode && <div className={styles.frame} />}
        <div className={styles.content}>
            {UIStore.inSelectionMode && UIStore.thumbnails && (
                <DisplaySelector thumbnails={UIStore.thumbnails} />
            )}
            {!UIStore.inSelectionMode && (
                <div className={styles.menu} style={{ maxHeight: HostHeight }}>
                    <span className={styles.statusText}>
                        You are currently sharing {UIStore.numDisplaysShared}{' '}
                        displays.
                    </span>
                    <button
                        onClick={() => startDesktopSelection()}
                        className={styles.changeSelectionButton}
                    >
                        Change Selection
                    </button>
                    <button
                        onClick={() =>
                            ipcRenderer.send(
                                RendererToMainIPCEvents.Host_DisconnectButton
                            )
                        }
                        className={styles.stopSharingButton}
                    >
                        Disconnect
                    </button>
                </div>
            )}
        </div>
    </div>
));

export default Host;
