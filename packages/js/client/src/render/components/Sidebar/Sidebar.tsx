import React from 'react';
import {
    faCog,
    faUserFriends,
    faDesktop,
} from '@fortawesome/free-solid-svg-icons';
import styles from './Sidebar.module.scss';
import logo from '../../../../../../../brand/render/logo.svg';
import SidebarItem from './SidebarItem';
import { Tab } from '../../store/UI';

const Sidebar: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.content}>
            <SidebarItem tab={Tab.CONNECT} icon={faDesktop}>
                Connect
            </SidebarItem>
            <SidebarItem tab={Tab.CONTACTS} icon={faUserFriends}>
                Contacts
            </SidebarItem>
            <SidebarItem tab={Tab.SETTINGS} icon={faCog}>
                Settings
            </SidebarItem>
        </div>
        <div className={styles.logoWrapper}>
            <img className={styles.logo} src={logo} />
            <span>ScreenView</span>
        </div>
    </div>
);
export default Sidebar;
