import React from 'react';
import styles from './Connect.module.scss';
import ConnectToMe from './ConnectToMe';
import DirectConnect from './DirectConnect';

const Connect: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.left}>
            <ConnectToMe />
        </div>
        <div className={styles.right}>
            <DirectConnect />
        </div>
    </div>
);
export default Connect;
