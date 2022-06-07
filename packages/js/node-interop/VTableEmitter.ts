import { EventEmitter } from 'events';
import { DisplayInformation, EstablishSessionStatus } from './index';
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
    WpsskaClientOutOfAuthenticationSchemes = 'wpsska_client_out_of_authentication_schemes',
    RvdDisplayUpdate = 'rvd_display_update',
    RvdFrameData = 'rvd_frame_data',
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
        event: VTableEvent.RvdDisplayUpdate,
        listener: (
            clipboardReadable: boolean,
            displays: DisplayInformation[]
        ) => void
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

    svsc_error_lease_extention_request_rejected() {
        this.emit(VTableEvent.SvscErrorLeaseExtentionRequestRejected);
    }

    /* wpskka - client */
    wpskka_client_password_prompt() {
        this.emit(VTableEvent.WpsskaClientPasswordPrompt);
    }

    wpskka_client_authentication_successful() {
        this.emit(VTableEvent.WpsskaClientAuthenticationSuccessful);
    }

    wpskka_client_out_of_authentication_schemes() {
        this.emit(VTableEvent.WpsskaClientOutOfAuthenticationSchemes);
    }

    /* rvd */

    rvd_frame_data(displayId: number, data: ArrayBuffer) {
        this.emit(VTableEvent.RvdFrameData, displayId, data);
    }

    rvd_display_update(
        clipboardReadable: boolean,
        displays: DisplayInformation[]
    ): void {
        this.emit(VTableEvent.RvdDisplayUpdate, clipboardReadable, displays);
    }
}

export default VTableEmitter;
