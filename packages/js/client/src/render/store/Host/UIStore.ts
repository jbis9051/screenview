import { makeAutoObservable } from 'mobx';
import { NativeThumbnail } from 'node-interop';

class UIStore {
    thumbnails: NativeThumbnail[] | null = null; // when not null, thumbnails are displayed

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
