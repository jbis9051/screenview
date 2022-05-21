import { ipcRenderer, IpcRendererEvent } from 'electron';
import { NativeThumbnail } from 'node-interop';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../common/IPCEvents';

export default function getDesktopList(): Promise<NativeThumbnail[]> {
    return new Promise((resolve, reject) => {
        ipcRenderer.send(RendererToMainIPCEvents.Host_GetDesktopList);

        function onDesktopList(
            _: IpcRendererEvent,
            list: NativeThumbnail[] | null
        ) {
            ipcRenderer.removeListener(
                MainToRendererIPCEvents.DesktopList,
                onDesktopList
            );
            if (list === null) {
                reject(new Error('Unable to get displays'));
                return;
            }
            resolve(list);
        }

        ipcRenderer.on(MainToRendererIPCEvents.DesktopList, onDesktopList);
    });
}
