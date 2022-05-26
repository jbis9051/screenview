export enum MainToRendererIPCEvents {
    VTableEvent = 'vtable-event',
    SessionId = 'session-id',
    DesktopList = 'desktop-list',
    Config = 'config',
}

export enum RendererToMainIPCEvents {
    Main_EstablishSession = 'main-establish-session',
    Host_GetDesktopList = 'host-get-desktop-list',
    Host_StopDesktopList = 'stop-get-desktop-list',
    Client_PasswordInput = 'client-password-input',
    Client_MouseInput = 'client-mouse-input',
    Client_KeyboardInput = 'client-keyboard-input',
    Main_ConfigUpdate = 'host-config-update',
}
