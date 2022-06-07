import React from 'react';
import { IconProp } from '@fortawesome/fontawesome-svg-core';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import cl from 'classnames';
import {
    faThLarge,
    faBinoculars,
    faArrowPointer,
    faSquare,
} from '@fortawesome/free-solid-svg-icons';
import { observer } from 'mobx-react';
import { action } from 'mobx';
import styles from './Client.module.scss';
import UIStore, { ViewMode } from '../../store/Client/UIStore';

const Control: React.FunctionComponent<{ label: string }> = ({
    children,
    label,
}) => (
    <div className={styles.control}>
        <div className={styles.top}>{children}</div>
        <div className={styles.bottom}>{label}</div>
    </div>
);

interface ToggleControlProps<T> {
    icons: Array<{ icon: IconProp; state: T }>;
    currentState: T;
    set: (state: T) => void;
}

const ToggleSwitch: React.FunctionComponent<ToggleControlProps<any>> = ({
    icons,
    currentState,
    set,
}) => (
    <div className={styles.toggleSwitch}>
        {icons.map(({ icon, state }) => (
            <div
                className={cl(styles.switch, {
                    [styles.active]: currentState === state,
                })}
                onClick={() => set(state)}
            >
                <FontAwesomeIcon icon={icon} />
            </div>
        ))}
    </div>
);

const Controls: React.FunctionComponent = observer(() => (
    <div className={styles.controls}>
        {UIStore.displayInformation.length > 1 && (
            <Control label={'View Mode'}>
                <ToggleSwitch
                    icons={[
                        {
                            icon: faThLarge,
                            state: ViewMode.Grid,
                        },
                        {
                            icon: faSquare,
                            state: ViewMode.Single,
                        },
                    ]}
                    currentState={UIStore.viewMode}
                    set={action((state) => {
                        UIStore.viewMode = state;
                    })}
                />
            </Control>
        )}
        {UIStore.controllable && (
            <Control label="Control">
                <ToggleSwitch
                    icons={[
                        {
                            icon: faArrowPointer,
                            state: true,
                        },
                        {
                            icon: faBinoculars,
                            state: false,
                        },
                    ]}
                    currentState={UIStore.controlling}
                    set={action((state) => {
                        UIStore.controlling = state;
                    })}
                />
            </Control>
        )}
    </div>
));

export default Controls;
