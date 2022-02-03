import React from 'react';

const Label: React.FunctionComponent<
    React.LabelHTMLAttributes<HTMLLabelElement>
> = ({ className, ...props }) => <label className={className} {...props} />;
export default Label;
