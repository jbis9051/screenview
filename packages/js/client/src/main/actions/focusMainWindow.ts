import { action, runInAction } from 'mobx';
import GlobalState from '../GlobalState';
import createMainWindow from '../factories/createMainWindow';

export default async function focusMainWindow(state: GlobalState) {
    if (state.mainWindow) {
        state.mainWindow.show();
        return;
    }
    const window = await createMainWindow();
    window.on(
        'close',
        action(() => {
            state.mainWindow = null;
        })
    );
    runInAction(() => {
        state.mainWindow = window;
    });
}
