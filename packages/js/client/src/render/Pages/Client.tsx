import React, { useState } from 'react';
import '../components/global.scss';
import styles from './Client.module.scss';

const Client: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.frame} />
        <div className={styles.canvasWrapper}></div>
    </div>
);

export default Client;
