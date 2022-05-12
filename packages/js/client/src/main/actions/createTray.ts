import path from 'path';
import GlobalState from '../GlobalState';
import Tray = Electron.Tray;

export default function createTray(state: GlobalState) {
    if (state.tray) {
        return;
    }

    state.tray = new Tray(
        path.resolve(
            path.join(__dirname, '/../../../../../brand/render/menu.png')
        )
    );
}
