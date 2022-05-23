import { makeAutoObservable } from 'mobx';
import { DisplayType, NativeThumbnail } from 'node-interop';

export interface SelectedDisplay {
    native_id: number;
    display_type: DisplayType;
}

class UIStore {
    thumbnails: NativeThumbnail[] | null = null; // when not null, thumbnails are displayed

    selectedDisplays: SelectedDisplay[] | null = null;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
