import { autorun } from 'mobx';
import { MainToRendererIPCEvents } from '../../common/IPCEvents';
import GlobalState from '../GlobalState';

export default function setupReactions(state: GlobalState) {
    autorun(() => {
        if (state.mainWindow) {
            state.mainWindow.webContents.send(
                MainToRendererIPCEvents.SessionIDChanged,
                state.sessionId
            );
        }
    });
}
