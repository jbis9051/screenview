import React from 'react';
import {
    faCog,
    faUser,
    faUserFriends,
    faDesktop,
} from '@fortawesome/free-solid-svg-icons';
import styles from './NavBar.module.scss';
import NavItem from './NavItem';
import { Tab } from '../../store/ToolBoxUIStore';

const NavBar: React.FunctionComponent = () => (
    <div className={styles.container}>
        <NavItem icon={faDesktop} tab={Tab.CONNECTION} />
    </div>
);
export default NavBar;
