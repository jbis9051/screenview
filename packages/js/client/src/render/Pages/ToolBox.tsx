import React from 'react';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { faXmark } from '@fortawesome/free-solid-svg-icons';
import styles from './ToolBox.module.scss';
import logo from '../../../../../../brand/render/logo.svg';
import NavBar from '../components/ToolBox/NavBar';

const ToolBox: React.FunctionComponent = () => (
    <div className={styles.wrapper}>
        <div className={styles.frame}>
            <div className={styles.logoWrapper}>
                <img className={styles.logo} src={logo} />
                <span>ScreenView</span>
                <div className={styles.x}>
                    <FontAwesomeIcon icon={faXmark} />
                </div>
            </div>
        </div>
        <NavBar />
    </div>
);
export default ToolBox;
