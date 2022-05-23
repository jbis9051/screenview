import path from 'path';
import { Tray } from 'electron';
import GlobalState from '../GlobalState';

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
