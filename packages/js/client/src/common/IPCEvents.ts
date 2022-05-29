export enum MainToRendererIPCEvents {
    VTableEvent = 'vtable-event',
    Main_SessionId = 'session-id',
    Host_DesktopList = 'desktop-list',
    Main_ConfigResponse = 'config',
}

export enum RendererToMainIPCEvents {
    Main_EstablishSession = 'main-establish-session',
    Main_ConfigUpdate = 'host-config-update',
    Main_ConfigRequest = 'host-config-request',
    Host_GetDesktopList = 'host-get-desktop-list',
    Host_StopDesktopList = 'stop-get-desktop-list',
    Host_UpdateDesktopList = 'host-desktop-list',
    Host_Disconnect = 'host-disconnect',
    Client_PasswordInput = 'client-password-input',
    Client_MouseInput = 'client-mouse-input',
    Client_KeyboardInput = 'client-keyboard-input',
}
