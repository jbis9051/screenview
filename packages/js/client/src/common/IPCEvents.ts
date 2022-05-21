export enum MainToRendererIPCEvents {
    VTableEvent = 'vtable-event',
    SessionId = 'session-id',
}
export enum RendererToMainIPCEvents {
    Main_EstablishSession = 'main-establish-session',
    Client_PasswordInput = 'client-password-input',
    Client_MouseInput = 'client-mouse-input',
    Client_KeyboardInput = 'client-keyboard-input',
}
