import { ipcRenderer, IpcRendererEvent } from 'electron';
import { action, runInAction, toJS } from 'mobx';
import { Display, NativeThumbnail } from 'node-interop';
import {
    MainToRendererIPCEvents,
    RendererToMainIPCEvents,
} from '../../../common/IPCEvents';
import UIStore from '../../store/Host/UIStore';

export default function getDesktopList(): Promise<Display[]> {
    return new Promise((resolve, reject) => {
        ipcRenderer.send(RendererToMainIPCEvents.Host_GetDesktopList);

        runInAction(() => {
            UIStore.inSelectionMode = true;
        });

        function onDesktopList(
            _: IpcRendererEvent,
            newList: NativeThumbnail[] | null
        ) {
            if (UIStore.selectedDisplays) {
                ipcRenderer.removeListener(
                    MainToRendererIPCEvents.Host_DesktopList,
                    onDesktopList
                );
                ipcRenderer.send(RendererToMainIPCEvents.Host_StopDesktopList);
                const selectedDisplays = toJS(UIStore.selectedDisplays);
                UIStore.numDisplaysShared = selectedDisplays.length; // TODO this is very much not the source of truth, we should have rust inform of how many displays are shared
                UIStore.selectedDisplays = null;
                UIStore.thumbnails = null;
                UIStore.inSelectionMode = false;
                resolve(selectedDisplays);
                return;
            }
            if (newList === null) {
                reject(new Error('Unable to get displays'));
                return;
            }
            runInAction(() => {
                if (!UIStore.thumbnails) {
                    UIStore.thumbnails = newList;
                    return;
                }
                UIStore.thumbnails = [
                    ...UIStore.thumbnails.filter(
                        (thumb) =>
                            !newList.find(
                                (newListItem) =>
                                    thumb.native_id === newListItem.native_id &&
                                    thumb.display_type ===
                                        newListItem.display_type
                            )
                    ),
                    ...newList,
                ].sort((a, b) => a.native_id - b.native_id);
            });
        }

        ipcRenderer.on(MainToRendererIPCEvents.Host_DesktopList, onDesktopList);
    });
}
