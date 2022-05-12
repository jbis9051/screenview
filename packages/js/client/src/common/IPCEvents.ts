export enum MainToRendererIPCEvents {
    SessionIDChanged = 'session-id-changed',
    SessionUpdate = 'session-update',
}
export enum RendererToMainIPCEvents {
    EstablishSession = 'establish-session',
}
