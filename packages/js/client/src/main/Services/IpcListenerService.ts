import { ipcMain, IpcMainEvent } from 'electron';

class Listener {
    constructor(
        public event: string,
        public cb: (event: IpcMainEvent, ...args: any[]) => void,
        public windowId?: number
    ) {}
}

export default class IpcListenerService {
    private listeners: Listener[] = [];

    private events: string[] = [];

    private destroyGen(listener: Listener) {
        return () => {
            this.listeners = this.listeners.filter((l) => l !== listener);
            if (this.listeners.find((l) => l.event === listener.event)) {
                return;
            }
            this.events = this.events.filter((e) => e !== listener.event);
            ipcMain.removeAllListeners(listener.event);
        };
    }

    genCallback(event: string) {
        return (e: IpcMainEvent, ...args: any[]) => {
            this.listeners.forEach((l) => {
                if (l.event === event) {
                    if (l.windowId && l.windowId !== e.sender.id) {
                        return;
                    }
                    l.cb(e, ...args);
                }
            });
        };
    }

    listen(
        event: string,
        cb: (event: IpcMainEvent, ...args: any[]) => void,
        windowId?: number
    ) {
        const listener = new Listener(event, cb, windowId);
        this.listeners.push(listener);
        if (!this.events.includes(event)) {
            this.events.push(event);
            ipcMain.on(event, this.genCallback(event));
        }
        return this.destroyGen(listener);
    }
}
