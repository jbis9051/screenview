import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
    VTableEvent,
} from '@screenview/node-interop';
import { action } from 'mobx';
import { BrowserWindow } from 'electron';
import createClientWindow from '../../factories/createClientWindow';
import { MainToRendererIPCEvents } from '../../../common/IPCEvents';
import GlobalState from '../../GlobalState';
import waitForReadySignal from '../waitForReadySignal';

async function startSignalSession(
    instance: rust.ClientSignalInstance,
    emitter: VTableEmitter,
    window: BrowserWindow,
    addr: string,
    state: GlobalState
) {
    await rust.connect(
        instance,
        ConnectionType.Reliable,
        state.config.signalServerReliable
    );
    await rust.connect(
        instance,
        ConnectionType.Unreliable,
        state.config.signalServerUnreliable
    );

    await rust.establish_session(instance, addr);
}

export default async function establishSession(state: GlobalState, id: string) {
    const emitter = new VTableEmitter();
    const window = await createClientWindow();
    const windowReadyPromise = waitForReadySignal(window);

    const formatted = id.replaceAll(/\s/g, '');

    const isSessionId = id.match(/^\d+$/);

    const instance = rust.new_instance(
        InstancePeerType.Client,
        isSessionId
            ? InstanceConnectionType.Signal
            : InstanceConnectionType.Direct,
        emitter
    ) as rust.ClientInstance;

    state.clientBundles.push({
        instance,
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

    try {
        if (isSessionId) {
            await startSignalSession(
                instance as rust.ClientSignalInstance,
                emitter,
                window,
                formatted,
                state
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
    } catch (e: any) {
        await windowReadyPromise;
        window.webContents.send(
            MainToRendererIPCEvents.Client_ConnectingFailed,
            e.toString()
        );
    }
}
