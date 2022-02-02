import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faRedoAlt } from '@fortawesome/free-solid-svg-icons';
import { observer } from 'mobx-react';
import styles from './ConnectToMe.module.scss';
import Title from './Title';
import BackendStore from '../../../store/BackendStore';
import Input from '../../Utility/Input';
import UIStore from '../../../store/UIStore';
import formatID from '../../../helper/formatID';

const ConnectToMe: React.FunctionComponent = observer(() => (
    <>
        <Title>Allow Connections</Title>
        <div className={styles.infoWrapper}>
            <div className={styles.infoName}>Your ID</div>
            <div className={styles.infoContent}>
                {(BackendStore.id && formatID(BackendStore.id)) || '-'}
            </div>
        </div>
        <div className={styles.infoWrapper}>
            <div className={styles.infoName}>Password</div>
            <div className={styles.infoContent}>
                {BackendStore.password || '-'}
                {BackendStore.password && (
                    <>
                        {formatID(BackendStore.password)}
                        <span className={styles.regenPassword}>
                            <FontAwesomeIcon icon={faRedoAlt} />
                        </span>
                    </>
                )}
            </div>
        </div>
        <label className={styles.label}>
            <Input
                type={'checkbox'}
                className={styles.checkbox}
                checked={UIStore.shareAllScreensImmediately}
                onChange={(e) => {
                    UIStore.shareAllScreensImmediately = e.target.checked;
                }}
            />
            Automatically Share My Desktop
        </label>
        <label className={styles.label}>
            <Input
                type={'checkbox'}
                className={styles.checkbox}
                checked={UIStore.allowControl}
                onChange={(e) => {
                    UIStore.allowControl = e.target.checked;
                }}
            />
            Allow Control
        </label>
    </>
));
export default ConnectToMe;
