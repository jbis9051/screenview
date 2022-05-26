import { action, runInAction, toJS } from 'mobx';
import GlobalState from '../GlobalState';
import createMainWindow from '../factories/createMainWindow';
import { MainToRendererIPCEvents } from '../../common/IPCEvents';

export default async function focusMainWindow(state: GlobalState) {
    if (state.mainWindow) {
        state.mainWindow.show();
        return;
    }
    const [pageLoad, window] = await createMainWindow();

    window.on(
        'close',
        action(() => {
            state.mainWindow = null;
        })
    );
    runInAction(() => {
        state.mainWindow = window;
    });

    pageLoad.then(() => {
        window.webContents.send(
            MainToRendererIPCEvents.Config,
            toJS(state.config)
        );
    });
}
