export enum MainToRendererIPCEvents {
    Main_SessionId = 'session-id',
    Host_DesktopList = 'desktop-list',
    Main_ConfigResponse = 'config',
    Client_ConnectingFailed = 'client-status-failed',
    Client_RvdDisplayUpdate = 'rvd-display-update',
}

export enum RendererToMainIPCEvents {
    Main_EstablishSession = 'main-establish-session',
    Main_ConfigUpdate = 'host-config-update',
    Main_ConfigRequest = 'host-config-request',
    Host_GetDesktopList = 'host-get-desktop-list',
    Host_StopDesktopList = 'stop-get-desktop-list',
    Host_UpdateDesktopList = 'host-desktop-list',
    Host_DisconnectButton = 'host-disconnect-button',
    Client_PasswordInput = 'client-password-input',
    Client_MouseInput = 'client-mouse-input',
    Client_KeyboardInput = 'client-keyboard-input',
    MacOSPPermission_Accessibility = 'mac-os-permission-accessibility',
    MacOSPPermission_ScreenCapture = 'mac-os-permission-screen-capture',
    MacOSPPermission_ScreenCapturePrompt = 'mac-os-permission-screen-capture-prompt',
    RendererReady = 'renderer-ready',
}
