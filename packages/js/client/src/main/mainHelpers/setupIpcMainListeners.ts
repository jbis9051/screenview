import { ipcMain } from 'electron';
import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from 'node-interop';
import { action } from 'mobx';
import GlobalState from '../GlobalState';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';
import VTableEmitter, { VTableEvent } from '../interopHelpers/VTableEmitter';
import createClientWindow from '../factories/createClientWindow';
import connectInstanceToSignal from '../interopHelpers/connectInstanceToSignal';

export default function setupIpcMainListeners(state: GlobalState) {
    ipcMain.on(
        RendererToMainIPCEvents.EstablishSession,
        async (_, id: string) => {
            const emitter = new VTableEmitter();
            const window = await createClientWindow();

            emitter.on(VTableEvent.SessionUpdate, () => {
                window.webContents.send(MainToRendererIPCEvents.SessionUpdate);
            });

            const formatted = id.replaceAll(/\s/g, '');

            const isSessionId = id.match(/^\d+$/);

            const instance = rust.new_instance(
                InstancePeerType.Client,
                isSessionId
                    ? InstanceConnectionType.Signal
                    : InstanceConnectionType.Direct,
                emitter
            );

            state.clientBundles.push({
                instance: instance as rust.ClientInstance,
                window,
                emitter,
            });

            window.on(
                'close',
                action(() => {
                    state.clientBundles = state.clientBundles.filter(
                        (bundle) => bundle.window !== window
                    );
                })
            );

            if (isSessionId) {
                await connectInstanceToSignal(
                    state,
                    instance as rust.ClientSignalInstance
                );
                await rust.establish_session(
                    instance as rust.ClientSignalInstance,
                    formatted
                );
            } else {
                await rust.connect(
                    instance as rust.ClientDirectInstance,
                    ConnectionType.Reliable,
                    formatted
                );
                await rust.connect(
                    instance as rust.ClientDirectInstance,
                    ConnectionType.Unreliable,
                    formatted
                );
            }
        }
    );
}
