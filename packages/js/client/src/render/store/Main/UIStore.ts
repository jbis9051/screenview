import { makeAutoObservable } from 'mobx';

export enum Tab {
    CONNECT,
    MY_COMPUTERS,
    CONTACTS,
    SETTINGS,
}

class UIStore {
    currentTab: Tab = Tab.CONNECT;

    shareAllScreensImmediately = true;

    allowControl = true;

    modal = {
        signIn: false,
    };

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UIStore();
