import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faRedoAlt } from '@fortawesome/free-solid-svg-icons';
import { observer } from 'mobx-react';
import { action } from 'mobx';
import styles from './ConnectToMe.module.scss';
import Title from './Title';
import BackendStore from '../../../../store/Main/BackendStore';
import Input from '../../../Utility/Input';
import UIStore from '../../../../store/Main/UIStore';
import formatID from '../../../../helper/Main/formatID';
import Label from '../../../Utility/Label';
import CheckBox from '../../../Utility/CheckBox';

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
        <div className={styles.label}>
            <CheckBox
                className={styles.checkbox}
                checked={UIStore.shareAllScreensImmediately}
                onChange={action((checked) => {
                    UIStore.shareAllScreensImmediately = checked;
                })}
            >
                Automatically Share My Desktop
            </CheckBox>
        </div>
        <CheckBox
            className={styles.checkbox}
            checked={UIStore.allowControl}
            onChange={action((checked) => {
                UIStore.allowControl = checked;
            })}
        >
            Allow Control
        </CheckBox>
    </>
));
export default ConnectToMe;
