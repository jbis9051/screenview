import React from 'react';
import { observer } from 'mobx-react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faUser } from '@fortawesome/free-solid-svg-icons';
import styles from './User.module.scss';
import UserStore from '../../store/UserStore';
import UIStore from '../../store/UIStore';
import ConfigStore from '../../store/ConfigStore';

const User: React.FunctionComponent = observer(() => (
    <div
        className={styles.wrapper}
        onClick={() => {
            if (!UserStore.user) {
                UIStore.modal.signIn = true;
            }
        }}
    >
        {UserStore.user || (
            <>
                <div className={styles.imageWrapper}>
                    <div className={styles.imageWrapperCircle}>
                        <FontAwesomeIcon icon={faUser} />
                    </div>
                </div>
                <div className={styles.signIn}>
                    <span>Sign In</span>
                    <a
                        onClick={(e) => e.stopPropagation()}
                        className={styles.signUp}
                        href={`${ConfigStore.authUrl}/auth/sign_up`}
                        target={'_blank'}
                    >
                        Don't have an account?
                    </a>
                </div>
            </>
        )}
    </div>
));
export default User;
