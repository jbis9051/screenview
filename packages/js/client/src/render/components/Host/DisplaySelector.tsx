import React, { useEffect, useState } from 'react';
import { observer } from 'mobx-react';
import cl from 'classnames';
import { NativeThumbnail } from 'node-interop';
import { action } from 'mobx';
import styles from './DisplaySelector.module.scss';
import interop from '../../nodeInterop';
import Button from '../Utility/Button';
import UIStore, { SelectedDisplay } from '../../store/Host/UIStore';

enum Tab {
    Monitors,
    Windows,
}

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

const DisplaySelector: React.FunctionComponent<{
    thumbnails: NativeThumbnail[];
}> = ({ thumbnails }) => {
    const [selected, setSelected] = useState<SelectedDisplay[]>([]);
    const [tab, setTab] = useState<Tab>(Tab.Monitors);
    const [thumbURLs, setThumbURLs] = useState<string[]>([]);

    const thumbs = thumbnails.filter(
        (nativeThumb) =>
            (tab === Tab.Monitors &&
                nativeThumb.display_type === interop.DisplayType.Monitor) ||
            (tab === Tab.Windows &&
                nativeThumb.display_type === interop.DisplayType.Window)
    );

    useEffect(() => {
        setThumbURLs(
            thumbs.map((nativeThumb) => {
                const arrayBufferView = new Uint8Array(nativeThumb.data);
                const blob = new Blob([arrayBufferView], {
                    type: 'image/jpeg',
                });
                return URL.createObjectURL(blob);
            })
        );
        return () => {
            thumbURLs.forEach((url) => {
                URL.revokeObjectURL(url);
            });
        };
    }, [thumbnails]);

    function toggleSelect(selectedDisplay: SelectedDisplay) {
        if (
            selected.find(
                (s) =>
                    s.native_id === selectedDisplay.native_id &&
                    s.display_type === selectedDisplay.display_type
            )
        ) {
            setSelected(
                selected.filter(
                    (s) =>
                        s.native_id !== selectedDisplay.native_id ||
                        s.display_type !== selectedDisplay.display_type
                )
            );
        } else {
            setSelected([...selected, selectedDisplay]);
        }
    }

    return (
        <div className={styles.container}>
            <div className={styles.tabSelectorWrapper}>
                <TabComp tab={Tab.Monitors} setTab={setTab} currentTab={tab}>
                    Monitors
                </TabComp>
                <TabComp tab={Tab.Windows} setTab={setTab} currentTab={tab}>
                    Windows
                </TabComp>
            </div>
            <div className={styles.thumbContainer}>
                <div className={styles.thumbsWrapper}>
                    {thumbs.map((nativeThumb, i) => {
                        const key =
                            nativeThumb.display_type + nativeThumb.native_id;
                        return (
                            <div
                                key={key}
                                className={cl(styles.thumb, {
                                    [styles.thumbSelected]: !!selected.find(
                                        (s) =>
                                            s.native_id ===
                                                nativeThumb.native_id &&
                                            s.display_type ===
                                                nativeThumb.display_type
                                    ),
                                })}
                                onClick={() => {
                                    toggleSelect({
                                        native_id: nativeThumb.native_id,
                                        display_type: nativeThumb.display_type,
                                    });
                                }}
                            >
                                <img
                                    className={styles.thumbImage}
                                    src={thumbURLs![i]}
                                />
                                <span className={styles.thumbName}>
                                    {nativeThumb.name}
                                </span>
                            </div>
                        );
                    })}
                </div>
            </div>
            <div className={styles.buttonContainer}>
                <Button
                    onClick={action(() => {
                        UIStore.selectedDisplays = selected;
                    })}
                >
                    Confirm
                </Button>
            </div>
        </div>
    );
};

export default DisplaySelector;
