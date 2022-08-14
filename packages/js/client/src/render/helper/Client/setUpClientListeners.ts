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
        console.log('Wpsska Client Authentication Success');
        UIStore.connectionStatus = ConnectionStatus.Handshaking;
    });

    ipcRenderer.on(
        MainToRendererIPCEvents.Client_VTableEvent,
        (_, event, ...args: any[]) => {
            switch (event) {
                case VTableEvent.WpsskaClientAuthenticationFailed:
                    // @ts-ignore
                    handleWpsskaClientAuthenticationFailed(...args);
                    break;
                case VTableEvent.WpsskaClientAuthenticationSuccessful:
                    handleWpsskaClientAuthenticationSuccess();
                    break;
                default:
                    console.log('Unknown VTable Event', event);
                    break;
            }
        }
    );
}
