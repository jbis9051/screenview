import React from 'react';
import cn from 'classnames';
import styles from './Canvas.module.scss';

const Canvas: React.FunctionComponent<
    React.CanvasHTMLAttributes<HTMLCanvasElement>
> = ({ className, ...props }) => (
    <canvas className={cn(styles.canvas, className)} {...props} />
);
export default Canvas;
