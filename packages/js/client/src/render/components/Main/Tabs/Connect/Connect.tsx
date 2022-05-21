import React from 'react';
import styles from './Connect.module.scss';
import ConnectToMe from './ConnectToMe';
import ConnectToOthers from './ConnectToOthers';

const Connect: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.left}>
            <ConnectToMe />
        </div>
        <div className={styles.right}>
            <ConnectToOthers />
        </div>
    </div>
);
export default Connect;
