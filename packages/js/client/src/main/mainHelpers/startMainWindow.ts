import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
    VTableEmitter,
    VTableEvent,
} from '@screenview/node-interop';
import { action, runInAction } from 'mobx';
import focusMainWindow from '../actions/focusMainWindow';
import GlobalState from '../GlobalState';
import createHostWindow from '../factories/createHostWindow';

function setupCommonEvents(
    state: GlobalState,
    vtable: VTableEmitter,
    type: InstanceConnectionType
) {
    vtable.on(
        VTableEvent.WpsskaClientAuthenticationSuccessful,
        action(async () => {
            const hostWindow = await createHostWindow(type);
            if (type === InstanceConnectionType.Direct) {
                state.directHostWindow = hostWindow;
            } else {
                state.signalHostWindow = hostWindow;
            }
        })
    );
}

export default async function startMainWindow(state: GlobalState) {
    await focusMainWindow(state);

    // permission setup is handled via reactions when state.<some host> changes

    if (!state.signalHostInstance && state.config.startAsSignalHost) {
        const vtable = new VTableEmitter();

        await setupCommonEvents(state, vtable, InstanceConnectionType.Signal);

        vtable.on(
            VTableEvent.SvscLeaseUpdate,
            action((id) => {
                state.sessionId = id;
            })
        );

        const instance = rust.new_instance(
            InstancePeerType.Host,
            InstanceConnectionType.Signal,
            vtable
        );

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

        runInAction(() => {
            state.signalHostInstance = instance;
        });

        await rust.lease_request(instance);
    }

    if (!state.directHostInstance && state.config.startAsDirectHost) {
        const vtable = new VTableEmitter();

        await setupCommonEvents(state, vtable, InstanceConnectionType.Direct);

        const instance = rust.new_instance(
            InstancePeerType.Host,
            InstanceConnectionType.Direct,
            vtable
        );

        runInAction(() => {
            state.directHostInstance = instance;
        });
    }
}
