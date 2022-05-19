import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faRedoAlt } from '@fortawesome/free-solid-svg-icons';
import { observer } from 'mobx-react';
import { action } from 'mobx';
import styles from './ConnectToMe.module.scss';
import Title from './Title';
import BackendStore from '../../../../store/BackendStore';
import Input from '../../../Utility/Input';
import UIStore from '../../../../store/UIStore';
import formatID from '../../../../helper/formatID';
import Label from '../../../Utility/Label';

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
            <Input
                type={'checkbox'}
                id={'connectToMe.shareDesktop'}
                className={styles.checkbox}
                checked={UIStore.shareAllScreensImmediately}
                onChange={action((e) => {
                    UIStore.shareAllScreensImmediately = e.target.checked;
                })}
            />
            <Label htmlFor={'connectToMe.shareDesktop'}>
                Automatically Share My Desktop
            </Label>
        </div>
        <div className={styles.label}>
            <Input
                type={'checkbox'}
                id={'connectToMe.allowControl'}
                className={styles.checkbox}
                checked={UIStore.allowControl}
                onChange={action((e) => {
                    UIStore.allowControl = e.target.checked;
                })}
            />
            <Label htmlFor={'connectToMe.allowControl'}>Allow Control</Label>
        </div>
    </>
));
export default ConnectToMe;
