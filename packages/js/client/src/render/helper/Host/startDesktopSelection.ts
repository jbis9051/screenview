import { ipcRenderer } from 'electron';
import { RendererToMainIPCEvents } from '../../../common/IPCEvents';
import getDesktopList from './getDesktopList';

export default async function startDesktopSelection() {
    const thumbs = await getDesktopList();
    ipcRenderer.send(RendererToMainIPCEvents.Host_UpdateDesktopList, thumbs);
}
