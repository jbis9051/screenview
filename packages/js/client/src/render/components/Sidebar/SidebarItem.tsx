import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import React from 'react';
import { IconProp } from '@fortawesome/fontawesome-svg-core';
import { observer } from 'mobx-react';
import cl from 'classnames';
import styles from './SidebarItem.module.scss';
import UI, { Tab } from '../../store/UIStore';

interface SidebarItemProps {
    icon: IconProp;
    tab: Tab;
}

const SidebarItem: React.FunctionComponent<SidebarItemProps> = observer(
    ({ icon, tab, children }) => (
        <div
            onClick={() => {
                UI.currentTab = tab;
            }}
            className={cl(styles.wrapper, {
                [styles.selected]: UI.currentTab === tab,
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
