use anyhow::anyhow;
use once_cell::sync::OnceCell;
use std::{
    convert::Infallible,
    env::{self, VarError},
    path::PathBuf,
    process::abort,
};
use time::{OffsetDateTime, UtcOffset};

const TCP_PORT_ENV_VAR: &str = "TCP_PORT";
const UDP_PORT_ENV_VAR: &str = "UDP_PORT";
const MAX_BLOCKING_THREADS_ENV_VAR: &str = "MAX_BLOCKING_THREADS";
const LOG_DIR_ENV_VAR: &str = "LOG_DIR";
const UTC_OFFSET_ENV_VAR: &str = "UTC_OFFSET";

// TODO: consider raising this number if we end up spawning a lot of blocking tasks
const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;
const DEFAULT_LOG_DIR: &str = "logs";

static GLOBAL_CONFIG: OnceCell<Config> = OnceCell::new();

pub struct Config {
    pub tcp_port: u16,
    pub udp_port: u16,
    pub max_blocking_threads: usize,
    pub log_dir: PathBuf,
    pub utc_offset: Option<UtcOffset>,
}

impl Config {
    #[inline]
    pub fn get() -> &'static Config {
        match GLOBAL_CONFIG.get() {
            Some(config) => config,
            // This branch should never be taken, but for robustness we fall back to lazy
            // initialization and only abort if that fails.
            None => Self::handle_config_not_present(),
        }
    }

    #[inline(never)]
    fn handle_config_not_present() -> &'static Config {
        eprintln!("Config unexpectedly not present. Falling back to lazy initialization");

        match Self::get_or_try_init() {
            Ok(config) => config,
            Err(error) => {
                eprintln!("Failed to lazily initialize config: {error}");
                abort()
            }
        }
    }

    #[inline]
    pub fn get_or_try_init() -> anyhow::Result<&'static Config> {
        GLOBAL_CONFIG.get_or_try_init(Self::from_env)
    }

    fn from_env() -> anyhow::Result<Self> {
        let tcp_port = Self::parse(TCP_PORT_ENV_VAR, |var| var.parse())?;
        let udp_port = Self::parse(UDP_PORT_ENV_VAR, |var| var.parse())?;

        let max_blocking_threads = Self::parse_or_default(
            MAX_BLOCKING_THREADS_ENV_VAR,
            |var| var.parse(),
            DEFAULT_MAX_BLOCKING_THREADS,
        )?;

        let log_dir = Self::parse_or_default_with(
            LOG_DIR_ENV_VAR,
            |var| Ok::<_, Infallible>(PathBuf::from(var)),
            || PathBuf::from(DEFAULT_LOG_DIR.to_owned()),
        )?;

        let utc_offset = Self::parse_or_default_with(
            UTC_OFFSET_ENV_VAR,
            |var| -> Result<_, anyhow::Error> {
                let hours = var.parse()?;
                UtcOffset::from_hms(hours, 0, 0)
                    .map(Some)
                    .map_err(Into::into)
            },
            || UtcOffset::local_offset_at(OffsetDateTime::now_utc()).ok(),
        )?;

        Ok(Self {
            tcp_port,
            udp_port,
            max_blocking_threads,
            log_dir,
            utc_offset,
        })
    }

    fn parse<T, F, E>(env_var: &str, parse: F) -> anyhow::Result<T>
    where
        F: FnOnce(String) -> Result<T, E>,
        E: Into<anyhow::Error>,
    {
        match env::var(env_var) {
            Ok(var) => parse(var).map_err(Into::into),
            Err(VarError::NotPresent) => Err(anyhow!("Missing required env var {env_var}")),
            Err(error @ VarError::NotUnicode(_)) => Err(Self::not_unicode(env_var, error)),
        }
    }

    fn parse_or_default<T, F, E>(env_var: &str, parse: F, default: T) -> anyhow::Result<T>
    where
        F: FnOnce(String) -> Result<T, E>,
        E: Into<anyhow::Error>,
    {
        Self::parse_or_default_with(env_var, parse, || default)
    }

    fn parse_or_default_with<T, F, E, D>(env_var: &str, parse: F, default: D) -> anyhow::Result<T>
    where
        F: FnOnce(String) -> Result<T, E>,
        E: Into<anyhow::Error>,
        D: FnOnce() -> T,
    {
        match env::var(env_var) {
            Ok(var) => parse(var).map_err(Into::into),
            Err(VarError::NotPresent) => Ok(default()),
            Err(error @ VarError::NotUnicode(_)) => Err(Self::not_unicode(env_var, error)),
        }
    }

    fn not_unicode(env_var: &str, error: VarError) -> anyhow::Error {
        anyhow!("Failed to parse env var {env_var}: {error}")
    }
}
