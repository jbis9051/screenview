import {
    ConnectionType,
    InstanceConnectionType,
    InstancePeerType,
    rust,
} from 'node-interop';
import { action, runInAction } from 'mobx';
import focusMainWindow from '../actions/focusMainWindow';
import VTableEmitter, { VTableEvent } from '../interopHelpers/VTableEmitter';
import GlobalState from '../GlobalState';

export default async function startMainWindow(state: GlobalState) {
    await focusMainWindow(state);

    // TODO add for main to node communication

    if (!state.signalHostInstance && state.config.startAsSignalHost) {
        const vtable = new VTableEmitter();

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
