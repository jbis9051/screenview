import { makeAutoObservable } from 'mobx';

export enum Tab {
    CONNECTION,
}

class ToolBoxUIStore {
    currentTab: Tab = Tab.CONNECTION;

    constructor() {
        makeAutoObservable(this);
    }
}
export default new ToolBoxUIStore();
