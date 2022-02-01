import { makeAutoObservable } from 'mobx';

export enum Tab {
    CONNECT,
    CONTACTS,
    SETTINGS,
}

class UI {
    currentTab: Tab = Tab.CONNECT;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new UI();
