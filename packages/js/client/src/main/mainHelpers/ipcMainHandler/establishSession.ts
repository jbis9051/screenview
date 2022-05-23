import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from 'node-interop';
import { action } from 'mobx';
import { BrowserWindow } from 'electron';
import VTableEmitter, { VTableEvent } from '../../interopHelpers/VTableEmitter';
import createClientWindow from '../../factories/createClientWindow';
import { MainToRendererIPCEvents } from '../../../common/IPCEvents';
import GlobalState from '../../GlobalState';

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

    [VTableEvent.SvscVersionBad, VTableEvent.SvscSessionEnd].forEach((e) => {
        emitter.on(e, () => {
            window.webContents.send(MainToRendererIPCEvents.VTableEvent, e);
        });
    });

    emitter.on(VTableEvent.SvscErrorSessionRequestRejected, (status) => {
        window.webContents.send(
            MainToRendererIPCEvents.VTableEvent,
            VTableEvent.SvscErrorSessionRequestRejected,
            status
        );
    });

    await rust.establish_session(instance, addr);
}

export default async function establishSession(state: GlobalState, id: string) {
    const emitter = new VTableEmitter();
    const window = await createClientWindow();

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

    emitter.on(VTableEvent.WpsskaClientPasswordPrompt, () => {
        window.webContents.send(
            MainToRendererIPCEvents.VTableEvent,
            VTableEvent.WpsskaClientPasswordPrompt
        );
    });

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
}
