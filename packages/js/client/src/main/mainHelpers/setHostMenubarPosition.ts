import { BrowserWindow, screen } from 'electron';
import { HostWidth } from '../../common/contants';

function getScreenFromBrowserWindow(window: BrowserWindow) {
    const winBounds = window.getBounds();
    return screen.getDisplayNearestPoint({
        x: winBounds.x,
        y: winBounds.y,
    });
}
export default function setHostMenubarPosition(hostWindow: BrowserWindow) {
    const windowScreen = getScreenFromBrowserWindow(hostWindow);
    const screenWidth = windowScreen.size.width;
    hostWindow.setPosition((screenWidth - HostWidth) / 2, 0, true);
}
