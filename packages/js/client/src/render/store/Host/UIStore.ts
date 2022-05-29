import { makeAutoObservable } from 'mobx';
import { NativeThumbnail, Display } from 'node-interop';

class UIStore {
    inSelectionMode = false;

    thumbnails: NativeThumbnail[] | null = null; // Internal, do not depend on

    selectedDisplays: Display[] | null = null; // Internal, do not depend on

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
