import { BrowserWindow, shell } from 'electron';
import PageType from '../render/Pages/PageType';

let toolBox: BrowserWindow | undefined;

async function createToolBox() {
    if (toolBox) {
        toolBox.show();
        return toolBox;
    }

    toolBox = new BrowserWindow({
        height: 400,
        width: 300,
        resizable: false,
        frame: false,
    });

    toolBox.on('close', () => {
        toolBox = undefined;
    });

    if (process.env.NODE_ENV === 'development') {
        await toolBox.loadURL(`http://localhost:8080/#${PageType.TOOLBOX}`);
    }

    return toolBox;
    // toolBox.loadFile(path.join(__dirname, '../index.html'));
}
export default createToolBox;
