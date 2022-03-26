import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { IconProp } from '@fortawesome/fontawesome-svg-core';
import { observer } from 'mobx-react';
import cl from 'classnames';
import styles from './NavItem.module.scss';
import ToolBoxUIStore, { Tab } from '../../store/ToolBoxUIStore';

interface NavItemProps {
    icon: IconProp;
    tab: Tab;
}

const NavItem: React.FunctionComponent<NavItemProps> = observer(
    ({ icon, tab, children }) => (
        <div
            onClick={() => {
                ToolBoxUIStore.currentTab = tab;
            }}
            className={cl(styles.wrapper, {
                [styles.selected]: ToolBoxUIStore.currentTab === tab,
            })}
        >
            <div className={styles.icon}>
                <FontAwesomeIcon icon={icon} />
            </div>
            <div className={styles.content}>{children}</div>
        </div>
    )
);
export default NavItem;
