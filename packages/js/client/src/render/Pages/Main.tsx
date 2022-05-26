import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import styles from './Main.module.scss';
import Sidebar from '../components/Main/Sidebar/Sidebar';
import UI, { Tab } from '../store/Main/UIStore';
import Connect from '../components/Main/Tabs/Connect/Connect';
import StatusBar from '../components/Main/StatusBar';
import Modal from '../components/Main/Modal/Modal';
import Settings from '../components/Main/Tabs/Settings/Settings';

const Main = observer(() => (
    <div className={styles.wrapper}>
        <Modal />
        <div className={styles.frame} />
        <div className={styles.content}>
            <div className={styles.sideBar}>
                <Sidebar />
            </div>
            <div className={styles.rightContent}>
                <div className={styles.mainContent}>
                    {UI.currentTab === Tab.CONNECT && <Connect />}
                    {UI.currentTab === Tab.SETTINGS && <Settings />}
                </div>
                <div className={styles.statusBottom}>
                    <StatusBar />
                </div>
            </div>
        </div>
    </div>
));

export default Main;
