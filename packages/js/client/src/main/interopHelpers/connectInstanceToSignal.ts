import { rust, ConnectionType } from 'node-interop';
import GlobalState from '../GlobalState';

export default async function connectInstanceToSignal(
    state: GlobalState,
    instance: rust.SignalInstance
) {
    await rust.connect(
        instance,
        ConnectionType.Reliable,
        state.config.signalServerReliable
    );
    await rust.connect(
        instance,
        ConnectionType.Reliable,
        state.config.signalServerUnreliable
    );
}
