import { EventEmitter } from 'events';
import { DisplayShare, EstablishSessionStatus } from './index';
import { VTable } from './index.node';

export enum VTableEvent {
    SvscVersionBad = 'svsc_version_bad',
    SvscLeaseUpdate = 'svsc_lease_update',
    SvscSessionUpdate = 'svsc_session_update',
    SvscSessionEnd = 'svsc_session_end',
    SvscErrorLeaseRequestRejected = 'svsc_error_lease_request_rejected',
    SvscErrorSessionRequestRejected = 'svsc_error_session_request_rejected',
    SvscErrorLeaseExtentionRequestRejected = 'svsc_error_lease_extention_request_rejected',
    WpsskaClientPasswordPrompt = 'wpsska_client_password_prompt',
    WpsskaClientAuthenticationSuccessful = 'wpsska_client_authentication_successful',
    WpsskaClientAuthenticationFailed = 'wpsska_client_authentication_failed',
    WpsskaHostAuthenticationSuccessful = 'wpsska_host_authentication_successful',
    RvdClientHandshakeComplete = 'rvd_client_handshake_complete',
    RvdDisplayShare = 'rvd_display_share',
    RvdDisplayUnshare = 'rvd_display_unshare',
    RvdClientFrameData = 'rvd_client_frame_data',
    RvdHostHandshakeComplete = 'rvd_host_handshake_complete',
}

export declare interface VTableEmitter extends EventEmitter {
    on(
        event: VTableEvent.SvscLeaseUpdate,
        listener: (sessionId: string) => void
    ): this;

    on(
        event: VTableEvent.SvscErrorSessionRequestRejected,
        listener: (status: EstablishSessionStatus) => void
    ): this;

    on(
        event: VTableEvent.RvdDisplayShare,
        listener: (display: DisplayShare) => void
    ): this;

    on(
        event: VTableEvent.RvdDisplayUnshare,
        listener: (display_id: number) => void
    ): this;

    on(event: VTableEvent, listener: () => void): this;
}

export class VTableEmitter extends EventEmitter implements VTable {
    emit(eventName: string | symbol, ...args: any[]): boolean {
        if (eventName !== 'event') {
            this.emit('event', eventName, ...args);
        }
        return super.emit(eventName, ...args);
    }

    /* svsc */
    svsc_version_bad() {
        this.emit(VTableEvent.SvscVersionBad);
    }

    svsc_lease_update(sessionId: string) {
        this.emit(VTableEvent.SvscLeaseUpdate, sessionId);
    }

    svsc_session_update() {
        this.emit(VTableEvent.SvscSessionUpdate);
    }

    svsc_session_end() {
        this.emit(VTableEvent.SvscSessionEnd);
    }

    svsc_error_lease_request_rejected() {
        this.emit(VTableEvent.SvscErrorLeaseRequestRejected);
    }

    svsc_error_session_request_rejected(status: EstablishSessionStatus) {
        this.emit(VTableEvent.SvscErrorSessionRequestRejected, status);
    }

    svsc_error_lease_extension_request_rejected() {
        this.emit(VTableEvent.SvscErrorLeaseExtentionRequestRejected);
    }

    /* wpskka - client */
    wpskka_client_password_prompt() {
        this.emit(VTableEvent.WpsskaClientPasswordPrompt);
    }

    wpskka_client_authentication_successful() {
        this.emit(VTableEvent.WpsskaClientAuthenticationSuccessful);
    }

    wpskka_client_authentication_failed() {
        this.emit(VTableEvent.WpsskaClientAuthenticationFailed);
    }

    /* wpskka - host */
    wpskka_host_authentication_successful() {
        this.emit(VTableEvent.WpsskaHostAuthenticationSuccessful);
    }

    /* rvd */
    rvd_client_handshake_complete() {
        this.emit(VTableEvent.RvdClientHandshakeComplete);
    }

    rvd_client_frame_data(
        displayId: number,
        width: number,
        height: number,
        data: ArrayBuffer
    ) {
        this.emit(
            VTableEvent.RvdClientFrameData,
            displayId,
            width,
            height,
            data
        );
    }

    rvd_host_handshake_complete() {
        this.emit(VTableEvent.RvdHostHandshakeComplete);
    }

    rvd_client_display_share(share: DisplayShare): void {
        this.emit(VTableEvent.RvdDisplayShare, share);
    }

    rvd_client_display_unshare(displayId: number): void {
        this.emit(VTableEvent.RvdDisplayUnshare, displayId);
    }
}

export default VTableEmitter;
