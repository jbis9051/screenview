import React from 'react';
import cn from 'classnames';
import styles from './CheckBox.module.scss';

const CheckBox: React.FunctionComponent<
    Omit<React.LabelHTMLAttributes<HTMLLabelElement>, 'onChange'> & {
        checked: boolean;
        onChange: (checked: boolean) => void;
    }
> = ({ className, checked, onChange, children, ...props }) => (
    <label className={cn(styles.checkbox, className)} {...props}>
        <input
            type={'checkbox'}
            checked={checked}
            onChange={(e) => onChange(e.target.checked)}
        />
        {children}
    </label>
);
export default CheckBox;
