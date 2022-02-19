import React, { useState } from 'react';
import '../components/global.scss';
import styles from './RemoterDisplay.module.scss';

const RemoteDisplay: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.frame} />
        <div className={styles.canvasWrapper}></div>
    </div>
);

export default RemoteDisplay;
