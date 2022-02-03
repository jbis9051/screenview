import React from 'react';
import styles from './SignIn.module.scss';
import Input from '../Utility/Input';
import Label from '../Utility/Label';
import Button from '../Utility/Button';
import UIStore from '../../store/UIStore';
import ConfigStore from '../../store/ConfigStore';

const SignIn: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <form>
            <div className={styles.label}>
                <Label className={styles.labelText}>Username</Label>
                <Input className={styles.input} name={'username'} />
            </div>
            <div>
                <Label className={styles.labelText}>Password</Label>
                <Input
                    className={styles.input}
                    name={'password'}
                    type={'password'}
                />
                <a
                    className={styles.forgotPassword}
                    target={'_blank'}
                    href={`${ConfigStore.authUrl}/auth/forgot_password`}
                >
                    Forgot Password?
                </a>
            </div>
            <div className={styles.buttonWrapper}>
                <Button
                    onClick={() => {
                        UIStore.modal.signIn = false;
                    }}
                    type={'button'}
                    className={styles.button}
                >
                    Cancel
                </Button>
                <Button type={'submit'} className={styles.button}>
                    Submit
                </Button>
            </div>
        </form>
    </div>
);
export default SignIn;
