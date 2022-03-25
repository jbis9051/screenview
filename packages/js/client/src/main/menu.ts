import { Menu, Tray } from 'electron';
import * as path from 'path';

import MenuItemConstructorOptions = Electron.MenuItemConstructorOptions;
import MenuItem = Electron.MenuItem;
import createMainWindow from './mainWindow';

let memoTray: Tray;

function createMenu(id: string | null) {
    const tray =
        memoTray ||
        new Tray(
            path.resolve(
                path.join(__dirname, '/../../../../../brand/render/menu.png')
            )
        );
    memoTray = tray;

    const menu: Array<MenuItemConstructorOptions | MenuItem> = [];

    if (id) {
        menu.push({
            label: `ID: ${id}`,
            enabled: false,
        });
    }

    tray.setContextMenu(
        Menu.buildFromTemplate([
            ...menu,
            {
                label: 'Open Window',
                click: () => {
                    createMainWindow();
                },
            },
        ])
    );
    // mainWindow.loadFile(path.join(__dirname, '../index.html'));
}

export default createMenu;
