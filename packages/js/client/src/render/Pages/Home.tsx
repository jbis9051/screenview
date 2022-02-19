import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import styles from './Home.module.scss';
import Sidebar from '../components/Home/Sidebar/Sidebar';
import UI, { Tab } from '../store/UIStore';
import Connect from '../components/Home/Tabs/Connect/Connect';
import StatusBar from '../components/Home/StatusBar';
import Modal from '../components/Home/Modal/Modal';

const Home = observer(() => (
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
                </div>
                <div className={styles.statusBottom}>
                    <StatusBar />
                </div>
            </div>
        </div>
    </div>
));

export default Home;
