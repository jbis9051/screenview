import React from 'react';
import styles from './ToolBox.module.scss';

const Toolbox: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.frame} />
        <div className={styles.nav}>
            <div className={styles.navItem}></div>
        </div>
        <div className={styles.content}>
            <div>Hi</div>
        </div>
    </div>
);
export default Toolbox;
