import React from 'react';
import ReactDOM from 'react-dom';
import { runInAction } from 'mobx';
import { ipcRenderer } from 'electron';
import Main from './Pages/Main';
import PageType from './Pages/PageType';
import Client from './Pages/Client';
import BackendStore from './store/Host/BackendStore';
import interop from './nodeInterop';
import Host from './Pages/Host';
import setupPreferences from './helper/setupPreferences';
import startDesktopSelection from './helper/Host/startDesktopSelection';

// we render different pages based on the hash aka # after the URL. This isn't dynamic meaning you can't change pages. This makes sense for our app.
(async () => {
    await setupPreferences();

    const page = window.location.hash.substring(1) as PageType;

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
                return <Host />;
            default:
                throw new Error('Cannot Find Page');
        }
    };

    ReactDOM.render(<Page pageType={page} />, document.getElementById('root'));

    switch (page) {
        case PageType.SignalHost:
        case PageType.DirectHost: {
            await runInAction(async () => {
                BackendStore.type =
                    page === PageType.DirectHost
                        ? interop.InstanceConnectionType.Direct
                        : interop.InstanceConnectionType.Signal;
            });
            await startDesktopSelection();
            break;
        }
        default:
    }
})();
