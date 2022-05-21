import React, { ReactNode } from 'react';
import { observer } from 'mobx-react';
import styles from './Modal.module.scss';
import UIStore from '../../../store/Main/UIStore';
import SignIn from './SignIn';

const Modal: React.FunctionComponent = observer(() => {
    function getModal(): ReactNode | null {
        if (UIStore.modal.signIn) {
            return <SignIn />;
        }
        return null;
    }
    const element = getModal();
    if (element) {
        return <div className={styles.modal}>{element}</div>;
    }
    return null;
});
export default Modal;
