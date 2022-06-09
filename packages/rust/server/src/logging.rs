use crate::Config as ServerConfig;
use flate2::{write::GzEncoder, Compression};
use log::*;
use log4rs::{
    append::{
        rolling_file::{
            policy::compound::{roll::Roll, trigger::size::SizeTrigger, CompoundPolicy},
            RollingFileAppender,
        },
        Append,
    },
    config::{Appender, Config as Log4rsConfig, Root},
    encode::{self, Encode},
    filter::{Filter, Response},
};
use once_cell::sync::Lazy;
use std::{
    borrow::Cow,
    fmt::{self, Debug, Display, Formatter},
    fs::{read_dir, remove_file, rename, File},
    io,
    io::{stdout, Write},
    path::{Component, Path, PathBuf},
    sync::Mutex,
    thread,
};
use termion::color;
use time::{
    format_description::{self, FormatItem},
    OffsetDateTime,
    UtcOffset,
};

pub static TIME_FORMAT: Lazy<Vec<FormatItem<'static>>> = Lazy::new(|| {
    format_description::parse("[hour repr:24]:[minute]:[second]")
        .expect("Invalid time format description")
});

pub static DATE_FORMAT: Lazy<Vec<FormatItem<'static>>> =
    Lazy::new(|| format_description::parse("[year]-[month]-[day]").expect("Invalid date format"));

const FILE_SIZE_LIMIT: u64 = 50_000_000;

#[cfg(debug_assertions)]
const LEVEL_FILTER: LevelFilter = LevelFilter::Debug;
#[cfg(not(debug_assertions))]
const LEVEL_FILTER: LevelFilter = LevelFilter::Info;

macro_rules! format_record {
    ($writer:expr, $record:expr) => {{
        let writer = $writer;
        let record = $record;
        let location = Location::from_record(record);

        writeln!(
            writer,
            "[{} {}{}{}]: {}",
            format_time(current_time()),
            record.metadata().level(),
            if matches!(location, Location::Some { .. }) {
                " "
            } else {
                ""
            },
            location,
            record.args()
        )
    }};
}

// Sets up log4rs customized for the minecraft server
pub fn init_logger(crate_name: &'static str) -> Result<(), anyhow::Error> {
    // Logs info to the console with colors and such
    let console = CustomConsoleAppender;

    // Logs to log files
    let log_file = RollingFileAppender::builder()
        .encoder(Box::new(LogEncoder))
        .build(
            latest_log_path(),
            Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(FILE_SIZE_LIMIT)),
                Box::new(CustomLogRoller::new()),
            )),
        )?;

    let crate_filter = CrateFilter { prefix: crate_name };

    // Build the log4rs config
    let config = Log4rsConfig::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(crate_filter))
                .build("console", Box::new(console)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(crate_filter))
                .build("log_file", Box::new(log_file)),
        )
        .build(
            Root::builder()
                .appender("console")
                .appender("log_file")
                .build(LEVEL_FILTER),
        )?;

    log4rs::init_config(config)?;

    Ok(())
}

// Called at the end of main, compresses the last log file
pub fn cleanup() {
    // There's no reason to handle an error here
    let _ = CustomLogRoller::new().roll_threaded(&latest_log_path(), false);
}

fn latest_log_path() -> PathBuf {
    [&*ServerConfig::get().log_dir, Path::new("latest.log")]
        .into_iter()
        .collect::<PathBuf>()
}

fn current_time() -> OffsetDateTime {
    try_localize(OffsetDateTime::now_utc())
}

pub fn try_localize(datetime: OffsetDateTime) -> OffsetDateTime {
    match ServerConfig::get().utc_offset {
        Some(offset) => datetime.to_offset(offset),
        None => match UtcOffset::local_offset_at(datetime) {
            Ok(offset) => datetime.to_offset(offset),
            Err(_) => datetime,
        },
    }
}

fn format_time(datetime: OffsetDateTime) -> String {
    match datetime.format(&*TIME_FORMAT) {
        Ok(formatted) => formatted,
        Err(_) => "??:??:??".to_owned(),
    }
}

// Only allow logging from out crate
#[derive(Debug, Clone, Copy)]
struct CrateFilter {
    #[allow(dead_code)]
    prefix: &'static str,
}

impl Filter for CrateFilter {
    #[cfg(debug_assertions)]
    fn filter(&self, record: &Record) -> Response {
        match record.module_path() {
            Some(path) =>
                if path.starts_with(self.prefix) {
                    Response::Accept
                } else {
                    Response::Reject
                },
            None => Response::Reject,
        }
    }

    #[cfg(not(debug_assertions))]
    fn filter(&self, _record: &Record) -> Response {
        Response::Neutral
    }
}

// Custom implementation for a console logger so that it doesn't mangle the user's commands
#[derive(Debug)]
struct CustomConsoleAppender;

impl Append for CustomConsoleAppender {
    fn append(&self, record: &Record) -> Result<(), anyhow::Error> {
        let mut writer = stdout().lock();
        match record.metadata().level() {
            Level::Error => write!(writer, "{}", color::Fg(color::Red))?,
            Level::Warn => write!(writer, "{}", color::Fg(color::LightYellow))?,
            Level::Debug => write!(writer, "{}", color::Fg(color::LightCyan))?,
            _ => write!(writer, "{}", color::Fg(color::Reset))?,
        }
        format_record!(&mut writer, record)?;
        write!(writer, "{}", color::Fg(color::Reset))?;
        Ok(())
    }

    fn flush(&self) {}
}

#[derive(Debug, Clone, Copy)]
struct LogRollerData {
    current_day_log_count: u32,
    current_day: u16,
}

#[derive(Debug)]
struct CustomLogRoller {
    name_info: Mutex<LogRollerData>, // current day, log count for today
}

impl CustomLogRoller {
    pub fn new() -> Self {
        let mut max_index = 0;

        if let Ok(paths) = read_dir(&ServerConfig::get().log_dir) {
            let today = current_time().format(&*DATE_FORMAT).unwrap_or_default();

            // Find the logs that match today's date and determine the highest index ({date}-{index}.log).
            for path in paths
                .flatten()
                .flat_map(|entry| entry.file_name().into_string())
                .filter(|name| name.starts_with(&today))
            {
                if let Some(index) = Self::index_from_path(&path) {
                    if index > max_index {
                        max_index = index;
                    }
                }
            }
        }

        CustomLogRoller {
            name_info: Mutex::new(LogRollerData {
                current_day_log_count: max_index,
                current_day: current_time().ordinal(),
            }),
        }
    }

    fn index_from_path(path: &str) -> Option<u32> {
        let dash_index = path.rfind('-')?;
        let dot_index = path.find('.')?;
        path.get(dash_index.saturating_add(1) .. dot_index)
            .and_then(|index| index.parse::<u32>().ok())
    }

    pub fn roll_threaded(&self, file: &Path, threaded: bool) -> Result<(), anyhow::Error> {
        let mut guard = match self.name_info.lock() {
            Ok(guard) => guard,

            // Since the mutex is privately managed and errors are handled correctly, this shouldn't be an issue
            Err(_) => unreachable!("Logger mutex poisoned."),
        };

        // Check to make sure the log name info is still accurate
        let local_datetime = current_time();
        if local_datetime.ordinal() != guard.current_day {
            guard.current_day = local_datetime.ordinal();
            guard.current_day_log_count = 1;
        } else {
            guard.current_day_log_count = guard
                .current_day_log_count
                .checked_add(1)
                .expect("Log count for the day exceeded");
        }

        let config = ServerConfig::get();

        // Rename the file in case it's large and will take a while to compress
        let log: PathBuf = [&*config.log_dir, Path::new("latest-tmp.log")]
            .into_iter()
            .collect();
        rename(file, &log)?;

        let output_file_name = format!(
            "{}-{}.log.gz",
            local_datetime.format(&*DATE_FORMAT)?,
            guard.current_day_log_count
        );
        let output: PathBuf = [config.log_dir.clone(), PathBuf::from(output_file_name)]
            .into_iter()
            .collect();


        drop(guard);

        if threaded {
            thread::spawn(move || {
                Self::try_compress_log(&log, &output);
            });
        } else {
            Self::try_compress_log(&log, &output);
        }

        Ok(())
    }

    // Attempts compress_log and prints an error if it fails
    fn try_compress_log(input_path: &Path, output_path: &Path) {
        if let Err(error) = Self::compress_log(input_path, output_path) {
            error!("Failed to compress log file: {}", error);
        }
    }

    // Takes the source file and compresses it, writing to the output path. Removes the source when done.
    fn compress_log(input_path: &Path, output_path: &Path) -> Result<(), io::Error> {
        let mut input = File::open(input_path)?;
        let mut output = GzEncoder::new(File::create(output_path)?, Compression::default());
        io::copy(&mut input, &mut output)?;
        drop(output.finish()?);
        drop(input); // This needs to occur before file deletion on some OS's
        remove_file(input_path)
    }
}

impl Roll for CustomLogRoller {
    fn roll(&self, file: &Path) -> Result<(), anyhow::Error> {
        self.roll_threaded(file, true)
    }
}

#[derive(Debug)]
struct LogEncoder;

impl Encode for LogEncoder {
    fn encode(&self, writer: &mut dyn encode::Write, record: &Record<'_>) -> anyhow::Result<()> {
        format_record!(writer, record).map_err(Into::into)
    }
}

enum Location<'a> {
    None,
    Some { file: Cow<'a, str>, line: u32 },
}

impl<'a> Location<'a> {
    fn from_record(record: &Record<'a>) -> Self {
        let (file, line) = match record.level() {
            Level::Info | Level::Warn => return Self::None,
            _ => match (record.file(), record.line()) {
                (Some(file), Some(line)) => (file, line),
                _ => return Self::None,
            },
        };

        let truncated = Path::new(file)
            .components()
            .skip_while(|component| {
                matches!(
                    component,
                    Component::Prefix(_)
                        | Component::RootDir
                        | Component::CurDir
                        | Component::ParentDir
                ) || component != &Component::Normal("src".as_ref())
            })
            .skip(1)
            .collect::<PathBuf>();
        match truncated.into_os_string().into_string() {
            Ok(string) => Self::Some {
                file: Cow::Owned(string),
                line,
            },
            Err(_) => Self::Some {
                file: Cow::Borrowed(file),
                line,
            },
        }
    }
}

impl<'a> Display for Location<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Some { file, line } => write!(f, "{}:{}", file, line),
        }
    }
}
