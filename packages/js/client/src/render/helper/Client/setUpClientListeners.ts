import { ipcRenderer } from 'electron';
import { action } from 'mobx';
import { VTableEvent } from '@screenview/node-interop';
import { MainToRendererIPCEvents } from '../../../common/IPCEvents';
import UIStore, { ConnectionStatus } from '../../store/Client/UIStore';

export default function setUpClientListeners() {
    const handleWpsskaClientAuthenticationFailed = action((error: string) => {
        UIStore.connectionStatus = ConnectionStatus.Error;
        UIStore.error = `An Error Occurred While connecting: ${error}`;
    });

    const handleWpsskaClientAuthenticationSuccess = action(() => {
        UIStore.connectionStatus = ConnectionStatus.Handshaking;
    });

    ipcRenderer.on(
        MainToRendererIPCEvents.Client_VTableEvent,
        (_, event, ...args: any[]) => {
            switch (event) {
                case VTableEvent.WpsskaClientAuthenticationFailed as const:
                    // @ts-ignore
                    handleWpsskaClientAuthenticationFailed(...args);
                    break;
                default:
                    break;
            }
        }
    );
}
