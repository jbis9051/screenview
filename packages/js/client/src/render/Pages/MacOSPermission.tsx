import React, { useEffect, useState } from 'react';
import '../components/global.scss';
import { ipcRenderer } from 'electron';
import cl from 'classnames';
import styles from './MacOSPermission.module.scss';
import { RendererToMainIPCEvents } from '../../common/IPCEvents';
import Button from '../components/Utility/Button';

const StatusIndicator: React.FunctionComponent<{ status: boolean }> = ({
    status,
}) => (
    <div className={styles.status}>
        <span className={cl(styles.indicator, { [styles.on]: status })} />
        {status ? 'Granted' : 'Denied'}
    </div>
);

const MacOSPermission: React.FunctionComponent = () => {
    const [screenCapturePermission, setScreenCapturePermission] =
        useState(false);
    const [accessibilityPermission, setAccessibilityPermission] =
        useState(false);

    useEffect(() => {
        let accessibility: NodeJS.Timeout | null = null;
        async function accessibilityCB() {
            const val = await ipcRenderer.invoke(
                RendererToMainIPCEvents.MacOSPPermission_Accessibility,
                false
            );
            setAccessibilityPermission(val);
            setTimeout(accessibilityCB, 500);
        }
        accessibility = setTimeout(accessibilityCB, 1000);

        let screen: NodeJS.Timeout | null = null;
        async function screenCB() {
            const val = await ipcRenderer.invoke(
                RendererToMainIPCEvents.MacOSPPermission_ScreenCapture
            );
            setScreenCapturePermission(val);
            setTimeout(screenCB, 500);
        }
        screen = setTimeout(screenCB, 1000);
        return () => {
            if (accessibility) {
                clearTimeout(accessibility);
            }
            if (screen) {
                clearTimeout(screen);
            }
        };
    }, []);

    return (
        <div className={styles.wrapper}>
            <div className={styles.frame}>Review System Access</div>
            <div className={styles.content}>
                <h1 className={styles.title}>Review System Access</h1>
                <p>
                    ScreenView requires your permission to access system
                    capabilities to provide functionality. The following
                    permissions are needed if you would like to share your
                    screen with others (Host).
                </p>
                <div className={styles.permissionContainer}>
                    <div className={styles.permissionWrapper}>
                        <div className={styles.permission}>
                            <div className={styles.left}>
                                <span className={styles.type}>
                                    Screen Recording
                                </span>
                            </div>
                            <div className={styles.right}>
                                <div>
                                    Screen Recording permission is required so
                                    remote users can view your screen.
                                </div>
                                <div className={styles.bottom}>
                                    <StatusIndicator
                                        status={screenCapturePermission}
                                    />
                                    <Button
                                        disabled={screenCapturePermission}
                                        className={styles.button}
                                        onClick={() =>
                                            ipcRenderer.invoke(
                                                RendererToMainIPCEvents.MacOSPPermission_ScreenCapturePrompt
                                            )
                                        }
                                    >
                                        Request Access
                                    </Button>
                                </div>
                            </div>
                        </div>

                        <div className={styles.permission}>
                            <div className={styles.left}>
                                <span className={styles.type}>
                                    Accessibility
                                </span>
                            </div>
                            <div className={styles.right}>
                                <div className={styles.top}>
                                    Accessibility permission is required so
                                    remote users can control your mouse.
                                </div>
                                <div className={styles.bottom}>
                                    <StatusIndicator
                                        status={accessibilityPermission}
                                    />
                                    <Button
                                        disabled={accessibilityPermission}
                                        className={styles.button}
                                        onClick={() =>
                                            ipcRenderer.invoke(
                                                RendererToMainIPCEvents.MacOSPPermission_Accessibility,
                                                true
                                            )
                                        }
                                    >
                                        Request Access
                                    </Button>
                                </div>
                            </div>
                        </div>

                        <Button
                            className={styles.closeButton}
                            onClick={() => {
                                window.close();
                            }}
                        >
                            Close
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
};

export default MacOSPermission;
