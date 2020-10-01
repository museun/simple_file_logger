use std::io::Write;

use std::error::Error;
use std::path::Path;

type Result = std::result::Result<(), Box<dyn Error + Send + Sync + 'static>>;

/// Create the file logger at this path.
///
/// This will:
/// * append to the file
/// * open and then the file for each write
pub fn append_transient(path: impl AsRef<Path>) -> Result {
    init(
        Kind::Transient(path.as_ref().to_path_buf()),
        log_filter_parse::Filters::from_env(),
    )
}

/// Create the file logger at this path.
///
/// This will:
/// * truncate the file initially
/// * open and then the file for each write
pub fn truncate_transient(path: impl AsRef<Path>) -> Result {
    let _ = std::fs::remove_file(path.as_ref());
    init(
        Kind::Transient(path.as_ref().to_path_buf()),
        log_filter_parse::Filters::from_env(),
    )
}

/// Create the file logger at this path.
///
/// This will:
/// * append to the file
/// * keep the file open ('locked') until the process exists.
pub fn append(path: impl AsRef<Path>) -> Result {
    init(
        Kind::KeepOpen(std::fs::File::open(path)?),
        log_filter_parse::Filters::from_env(),
    )
}

/// Create the file logger at this path.
///
/// This will:
/// * truncate the file initially
/// * keep the file open ('locked') until the process exists.
pub fn truncate(path: impl AsRef<Path>) -> Result {
    init(
        Kind::KeepOpen(std::fs::File::open(path)?),
        log_filter_parse::Filters::from_env(),
    )
}

fn init(kind: Kind, filters: log_filter_parse::Filters) -> Result {
    log::set_max_level(log::LevelFilter::Trace);
    log::set_boxed_logger(Box::new(FileLogger { kind, filters }))?;
    Ok(())
}

struct FileLogger {
    kind: Kind,
    filters: log_filter_parse::Filters,
}

impl FileLogger {
    fn print(&self, record: &log::Record) {
        let (mut file, mut new);

        let write: &mut dyn std::io::Write = match &self.kind {
            Kind::KeepOpen(fi) => {
                file = fi;
                &mut file
            }
            Kind::Transient(path) => {
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    Ok(fi) => {
                        new = fi;
                        &mut new
                    }
                    Err(..) => return,
                }
            }
        };

        let _ = { write }.write_fmt(format_args!(
            "[{level: <5}] {timestamp} [{target}] {payload}",
            level = record.level(),
            timestamp = timestamp(),
            target = record.target(),
            payload = record.args(),
        ));
    }
}

impl log::Log for FileLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.filters.is_enabled(metadata)
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            self.print(record)
        }
    }

    #[inline]
    fn flush(&self) {
        if let Kind::KeepOpen(file) = &self.kind {
            let _ = { file }.flush();
        }
    }
}

enum Kind {
    KeepOpen(std::fs::File),
    Transient(std::path::PathBuf),
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as _
}
