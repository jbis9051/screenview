import GlobalState from '../GlobalState';
import createMainWindow from '../factories/createMainWindow';

export default async function focusMainWindow(state: GlobalState) {
    if (state.mainWindow) {
        state.mainWindow.show();
    } else {
        state.mainWindow = await createMainWindow();
    }
}
