import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import { toJS } from 'mobx';
import styles from './Host.module.scss';
import UIStore from '../store/Host/UIStore';
import DisplaySelector from '../components/Host/DisplaySelector';

const Host = observer(() => (
    <div className={styles.wrapper}>
        <div className={styles.frame} />
        <div className={styles.content}>
            {UIStore.thumbnails && (
                <DisplaySelector thumbnails={UIStore.thumbnails} />
            )}
        </div>
    </div>
));

export default Host;
