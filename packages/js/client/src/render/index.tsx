import React from 'react';
import ReactDOM from 'react-dom';
import { runInAction } from 'mobx';
import Main from './Pages/Main';
import PageType from './Pages/PageType';
import Client from './Pages/Client';
import BackendStore from './store/Host/BackendStore';
import interop from './nodeInterop';

// we render different pages based on the hash aka # after the URL. This isn't dynamic meaning you can't change pages. This makes sense for our app.
(async () => {
    const page = window.location.hash.substring(1) as PageType;

    switch (page) {
        case PageType.SignalHost:
        case PageType.DirectHost: {
            await runInAction(async () => {
                BackendStore.type =
                    page === PageType.DirectHost
                        ? interop.InstanceConnectionType.Direct
                        : interop.InstanceConnectionType.Signal;
                //  UIStore.thumbnails = await getDesktopList();
            });
            break;
        }
        default:
    }

    const Page: React.FunctionComponent<{ pageType: PageType }> = ({
        pageType,
    }) => {
        switch (pageType) {
            case PageType.Main:
                return <Main />;
            case PageType.Client:
                return <Client />;
            case PageType.SignalHost:
            case PageType.DirectHost:
                return null;
            default:
                throw new Error('Cannot Find Page');
        }
    };

    ReactDOM.render(<Page pageType={page} />, document.getElementById('root'));
})();
