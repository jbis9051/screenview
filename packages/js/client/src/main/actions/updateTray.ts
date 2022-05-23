import { Menu } from 'electron';
import MenuItemConstructorOptions = Electron.MenuItemConstructorOptions;
import MenuItem = Electron.MenuItem;
import GlobalState from '../GlobalState';

export default function updateTray(
    state: GlobalState,
    menu: Array<MenuItemConstructorOptions | MenuItem>
) {
    state.tray?.setContextMenu(Menu.buildFromTemplate(menu));
}
