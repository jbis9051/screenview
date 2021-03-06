import { makeAutoObservable } from 'mobx';
import { NativeThumbnail, Display } from '@screenview/node-interop';

class UIStore {
    inSelectionMode = false;

    thumbnails: NativeThumbnail[] | null = null; // Internal, do not depend on

    selectedDisplays: Display[] | null = null; // Internal, do not depend on

    numDisplaysShared = 0;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
