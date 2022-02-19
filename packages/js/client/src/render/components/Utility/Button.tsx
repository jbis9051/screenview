import React from 'react';
import cn from 'classnames';
import styles from './Button.module.scss';

const Button: React.FunctionComponent<
    React.ButtonHTMLAttributes<HTMLButtonElement>
> = ({ className, ...props }) => (
    <button className={cn(styles.button, className)} {...props} />
);
export default Button;
