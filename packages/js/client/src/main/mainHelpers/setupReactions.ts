import { autorun } from 'mobx';
import { rust } from 'node-interop';
import { MainToRendererIPCEvents } from '../../common/IPCEvents';
import GlobalState from '../GlobalState';

export default function setupReactions(state: GlobalState) {
    // this reaction updates the session id when the global state changes
    autorun(() => {
        if (state.mainWindow) {
            state.mainWindow.webContents.send(
                MainToRendererIPCEvents.SessionId,
                state.sessionId
            );
        }
    });

    // this reaction updates the permissions when the global state changes
    autorun(() => {
        [state.directHostInstance, state.signalHostInstance].forEach(
            async (instance) => {
                if (!instance) {
                    return;
                }
                await rust.update_static_password(
                    instance,
                    state.config.staticPassword
                );
                await rust.set_controllable(
                    instance,
                    state.config.isControllable
                );
                await rust.set_clipboard_readable(
                    instance,
                    state.config.isClipboardReadable
                );
            }
        );
    });
}
