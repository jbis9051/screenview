import { rust, EstablishSessionStatus } from 'node-interop';
import EventEmitter from 'events';

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
    RvdFrameData = 'rvd_frame_data',
}

interface Event {
    event: VTableEvent;
    data: any[];
}

class VTableMocker implements rust.VTable {
    emitter: EventEmitter = new EventEmitter();

    wait_for_event(event: VTableEvent, timeout = -1): Promise<any[]> {
        return new Promise((resolve) => {
            let expireTimeout: NodeJS.Timeout | null = null;
            const handleEvent = (e: Event) => {
                if (expireTimeout) {
                    clearTimeout(expireTimeout);
                }
                if (e.event !== event) {
                    throw new Error(`Expected event ${event}, got ${e.event}`);
                }
                resolve(e.data);
            };
            this.emitter.once(event, handleEvent);
            if (timeout > 0) {
                expireTimeout = setTimeout(() => {
                    this.emitter.removeListener(event, handleEvent);
                    throw new Error(`Timeout waiting for event ${event}`);
                }, timeout);
            }
        });
    }

    /* svsc */
    svsc_version_bad() {
        this.emitter.emit('event', {
            event: VTableEvent.SvscVersionBad,
            data: [],
        });
    }

    svsc_lease_update(sessionId: string) {
        this.emitter.emit('event', {
            event: VTableEvent.SvscLeaseUpdate,
            data: [sessionId],
        });
    }

    svsc_session_update() {
        this.emitter.emit('event', {
            event: VTableEvent.SvscSessionUpdate,
            data: [],
        });
    }

    svsc_session_end() {
        this.emitter.emit('event', {
            event: VTableEvent.SvscSessionEnd,
            data: [],
        });
    }

    svsc_error_lease_request_rejected() {
        this.emitter.emit('event', {
            event: VTableEvent.SvscErrorLeaseRequestRejected,
            data: [],
        });
    }

    svsc_error_session_request_rejected(status: EstablishSessionStatus) {
        this.emitter.emit('event', {
            event: VTableEvent.SvscErrorSessionRequestRejected,
            data: [status],
        });
    }

    svsc_error_lease_extention_request_rejected() {
        this.emitter.emit('event', {
            event: VTableEvent.SvscErrorLeaseExtentionRequestRejected,
            data: [],
        });
    }

    /* wpskka - client */
    wpskka_client_password_prompt() {
        this.emitter.emit('event', {
            event: VTableEvent.WpsskaClientPasswordPrompt,
            data: [],
        });
    }

    wpskka_client_authentication_successful() {
        this.emitter.emit('event', {
            event: VTableEvent.WpsskaClientAuthenticationSuccessful,
            data: [],
        });
    }

    wpskka_client_out_of_authentication_schemes() {
        this.emitter.emit('event', {
            event: VTableEvent.WpsskaClientOutOfAuthenticationSchemes,
            data: [],
        });
    }

    /* rvd */

    rvd_frame_data(displayId: number, data: ArrayBuffer) {
        this.emitter.emit('event', {
            event: VTableEvent.RvdFrameData,
            data: [displayId, data],
        });
    }
}

export default VTableMocker;
