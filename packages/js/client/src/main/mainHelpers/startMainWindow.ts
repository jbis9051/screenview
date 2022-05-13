import { rust, InstanceConnectionType, InstancePeerType } from 'node-interop';
import focusMainWindow from '../actions/focusMainWindow';
import VTableEmitter from '../interopHelpers/VTableEmitter';
import connectInstanceToSignal from '../interopHelpers/connectInstanceToSignal';
import GlobalState from '../GlobalState';

export default async function startMainWindow(state: GlobalState) {
    await focusMainWindow(state);

    // TODO add for main to node communication

    if (!state.signalHostInstance && state.config.startAsSignalHost) {
        const vtable = new VTableEmitter();

        vtable.on('session_id_update', (id) => {
            state.sessionId = id;
        });

        const instance = rust.new_instance(
            InstancePeerType.Host,
            InstanceConnectionType.Signal,
            vtable
        );

        await connectInstanceToSignal(state, instance);

        await rust.lease_request(instance); // TODO instance will now emit a lease response event eventually
        state.signalHostInstance = instance;
    }

    if (!state.directHostInstance && state.config.startAsDirectHost) {
        state.directHostInstance = rust.new_instance(
            InstancePeerType.Host,
            InstanceConnectionType.Direct,
            new VTableEmitter()
        );
    }
}
