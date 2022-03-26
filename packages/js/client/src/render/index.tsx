import React from 'react';
import ReactDOM from 'react-dom';
import Home from './Pages/Home';
import PageType from './Pages/PageType';
import RemoteDisplay from './Pages/RemoteDisplay';
import ToolBox from './Pages/ToolBox';

// we render different pages based on the hash aka # after the URL. This isn't dynamic meaning you can't change pages. This makes sense for our app.

const Page: React.FunctionComponent = () => {
    const page = window.location.hash.substring(1) as PageType;
    switch (page) {
        case PageType.HOME:
            return <Home />;
        case PageType.REMOTE_DISPLAY:
            return <RemoteDisplay />;
        case PageType.TOOLBOX:
            return <ToolBox />;
        default:
            throw new Error('Cannot Find Page');
    }
};

ReactDOM.render(<Page />, document.getElementById('root'));
