import React from 'react';
import '../components/global.scss';
import { observer } from 'mobx-react';
import styles from './Host.module.scss';
import UIStore from '../store/Host/UIStore';

const Host = observer(() => (
    <div className={styles.wrapper}>
        <div className={styles.frame} />
        <div className={styles.content}>
            <pre>
                {UIStore.thumbnails?.map((thumb) => {
                    const arrayBufferView = new Uint8Array(thumb.data);
                    const blob = new Blob([arrayBufferView], {
                        type: 'image/jpeg',
                    });
                    const url = URL.createObjectURL(blob);
                    return (
                        <img
                            src={url}
                            key={thumb.display_type + thumb.native_id}
                        />
                    );
                })}
            </pre>
        </div>
    </div>
));

export default Host;
