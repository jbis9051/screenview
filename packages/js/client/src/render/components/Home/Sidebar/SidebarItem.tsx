import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { IconProp } from '@fortawesome/fontawesome-svg-core';
import { observer } from 'mobx-react';
import cl from 'classnames';
import { faDesktop } from '@fortawesome/free-solid-svg-icons';
import styles from './SidebarItem.module.scss';
import UIStore, { Tab } from '../../../store/UIStore';
import UserStore from '../../../store/UserStore';

interface SidebarItemProps {
    icon: IconProp;
    tab: Tab;
    userGated?: boolean;
}

const SidebarItem: React.FunctionComponent<SidebarItemProps> = observer(
    ({ icon, tab, userGated = false, children }) => (
        <div
            onClick={() => {
                if (userGated && !UserStore.user) {
                    UIStore.modal.signIn = true;
                } else {
                    UIStore.currentTab = tab;
                }
            }}
            className={cl(styles.wrapper, {
                [styles.selected]: UIStore.currentTab === tab,
            })}
        >
            <div className={styles.icon}>
                <FontAwesomeIcon icon={icon} />
            </div>
            <div className={styles.content}>{children}</div>
        </div>
    )
);
export default SidebarItem;
