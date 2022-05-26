import React, { useMemo, useState } from 'react';
import { action } from 'mobx';
import cl from 'classnames';
import { observer } from 'mobx-react';
import styles from './Settings.module.scss';
import ConfigStore from '../../../../store/Main/ConfigStore';
import Button from '../../../Utility/Button';
import CheckBox from '../../../Utility/CheckBox';
import Input from '../../../Utility/Input';

enum Tab {
    General = 'General',
    Host = 'Host',
    Client = 'Client',
}

const StandardInput: React.FunctionComponent<
    React.InputHTMLAttributes<HTMLInputElement>
> = ({ children, className, ...props }) => (
    <label className={cl(styles.inputLabel, className)}>
        <span>{children}</span>
        <Input {...props} />
    </label>
);

const TabComp: React.FunctionComponent<{
    tab: Tab;
    currentTab: Tab;
    setTab: (tab: Tab) => void;
    children: React.ReactNode;
}> = ({ tab, currentTab, setTab, children }) => (
    <div
        className={cl(styles.tab, {
            [styles.tabSelected]: tab === currentTab,
        })}
        onClick={() => {
            setTab(tab);
        }}
    >
        {children}
    </div>
);

const Settings: React.FunctionComponent = observer(() => {
    const [tab, setTab] = useState(Tab.General);
    const [localConfig, setLocalConfig] = useState({
        ...ConfigStore,
        backend: { ...ConfigStore.backend },
    });

    const changed = !ConfigStore.equals(localConfig);

    function setBackend(backend: Partial<typeof localConfig.backend>) {
        setLocalConfig({
            ...localConfig,
            backend: {
                ...localConfig.backend,
                ...backend,
            },
        });
    }
    return (
        <div className={styles.container}>
            <div className={styles.tabContainer}>
                {Object.values(Tab).map((t) => (
                    <TabComp key={t} tab={t} setTab={setTab} currentTab={tab}>
                        {t}
                    </TabComp>
                ))}
            </div>
            <div className={styles.content}>
                {tab === Tab.General && (
                    <>
                        <StandardInput
                            className={styles.inputInput}
                            value={localConfig.backend.signalServerReliable}
                            onChange={(e) => {
                                setBackend({
                                    signalServerReliable: e.target.value,
                                });
                            }}
                        >
                            Signal Server Reliable
                        </StandardInput>
                        <StandardInput
                            className={styles.inputInput}
                            value={localConfig.backend.signalServerUnreliable}
                            onChange={(e) => {
                                setBackend({
                                    signalServerUnreliable: e.target.value,
                                });
                            }}
                        >
                            Signal Server Unreliable
                        </StandardInput>
                    </>
                )}
                {tab === Tab.Host && (
                    <>
                        <CheckBox
                            className={[styles.input, styles.checkbox].join(
                                ' '
                            )}
                            checked={localConfig.backend.startAsDirectHost}
                            onChange={(checked) =>
                                setBackend({ startAsDirectHost: checked })
                            }
                        >
                            Start as direct host
                        </CheckBox>
                        <CheckBox
                            className={[styles.input, styles.checkbox].join(
                                ' '
                            )}
                            checked={localConfig.backend.startAsSignalHost}
                            onChange={(checked) =>
                                setBackend({ startAsSignalHost: checked })
                            }
                        >
                            Start as signal host
                        </CheckBox>
                        <StandardInput
                            className={styles.inputInput}
                            placeholder={'Disabled'}
                            value={localConfig.backend.staticPassword || ''}
                            type={'password'}
                            onChange={(e) => {
                                setBackend({
                                    staticPassword:
                                        e.target.value.trim() === ''
                                            ? null
                                            : e.target.value.trim(),
                                });
                            }}
                        >
                            Static Password
                        </StandardInput>
                    </>
                )}
                {tab === Tab.Client && <>Client</>}
            </div>
            <div className={styles.buttonContainer}>
                <Button
                    className={styles.button}
                    disabled={!changed}
                    onClick={() => setLocalConfig(ConfigStore)}
                >
                    Revert
                </Button>
                <Button
                    disabled={!changed}
                    className={styles.button}
                    onClick={action(() => {
                        Object.keys(localConfig).forEach((key) => {
                            const akey = key as keyof typeof ConfigStore;
                            // @ts-ignore
                            ConfigStore[akey] = localConfig[akey];
                        });
                    })}
                >
                    Apply
                </Button>
            </div>
        </div>
    );
});

export default Settings;
