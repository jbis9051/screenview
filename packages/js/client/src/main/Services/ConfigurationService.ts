import Config from '../../common/Config';

export default interface ConfigurationService {
    load(): Promise<Config>;

    save(config: Config): Promise<void>;
}
