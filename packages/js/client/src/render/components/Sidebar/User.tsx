import React from 'react';
import { observer } from 'mobx-react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faUser } from '@fortawesome/free-solid-svg-icons';
import styles from './User.module.scss';
import UserStore from '../../store/UserStore';
import Button from '../Utility/Button';

const User: React.FunctionComponent = observer(() => (
    <div className={styles.wrapper}>
        {UserStore.user || (
            <>
                <div className={styles.imageWrapper}>
                    <div className={styles.imageWrapperCircle}>
                        <FontAwesomeIcon icon={faUser} />
                    </div>
                </div>
                <div className={styles.signIn}>
                    <span>Sign In</span>
                    <span className={styles.signUp}>
                        Don't have an account?
                    </span>
                </div>
            </>
        )}
    </div>
));
export default User;
