import React from 'react';
import cn from 'classnames';
import styles from './Input.module.scss';

const Input: React.FunctionComponent<
    React.InputHTMLAttributes<HTMLInputElement>
> = ({ className, ...props }) => (
    <input className={cn(styles.input, className)} {...props} />
);
export default Input;
