import React from 'react';
import cl from 'classnames';
import { observer } from 'mobx-react';
import styles from './StatusBar.module.scss';
import Backend from '../store/BackendStore';

const StatusBar: React.FunctionComponent = observer(() => (
    <div className={styles.wrapper}>
        <div
            className={cl(styles.status, { [styles.ready]: Backend.status })}
        />
        <div className={styles.text}>{!Backend.status && 'Not Ready'}</div>
    </div>
));
export default StatusBar;
