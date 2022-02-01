import React from 'react';
import './global.scss';
import { observer } from 'mobx-react';
import styles from './App.module.scss';
import Sidebar from './Sidebar/Sidebar';
import UI, { Tab } from '../store/UI';
import Connect from './Tabs/Connect/Connect';
import StatusBar from './StatusBar';

const App = observer(() => (
    <div className={styles.wrapper}>
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

export default App;
