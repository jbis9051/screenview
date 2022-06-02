import { ipcRenderer } from 'electron';
import { action } from 'mobx';
import { MainToRendererIPCEvents } from '../../../common/IPCEvents';
import UIStore, { ConnectionStatus } from '../../store/Client/UIStore';

export default function setUpClientListeners() {
    ipcRenderer.on(
        MainToRendererIPCEvents.Client_ConnectingFailed,
        action((_, error) => {
            UIStore.connectionStatus = ConnectionStatus.Error;
            UIStore.error = `An Error Occurred While connecting: ${error}`;
        })
    );
}
