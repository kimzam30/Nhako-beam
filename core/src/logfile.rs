//! The log file: recent logs kept in the config folder (`logs/nhako-beam.log`, plus one older
//! file), so a problem on someone's computer or phone can be looked at afterwards. The apps add it
//! to their logging, and Settings > About > Copy diagnostics includes its last lines.
//! Nothing is sent anywhere.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

/// Each file's size limit. One older file is kept, so logs take at most twice this.
pub const MAX_BYTES: u64 = 1024 * 1024;

pub struct LogFile {
    path: PathBuf,
    old: PathBuf,
    open: Mutex<Option<(File, u64)>>,
}

static INSTALLED: OnceLock<Arc<LogFile>> = OnceLock::new();

/// This process's log file in `<config_dir>/logs/`. Later calls return the first one.
pub fn install(config_dir: &Path) -> Arc<LogFile> {
    INSTALLED.get_or_init(|| Arc::new(LogFile::new(&config_dir.join("logs")))).clone()
}

/// The log file, if the app installed one.
pub fn installed() -> Option<Arc<LogFile>> {
    INSTALLED.get().cloned()
}

impl LogFile {
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join("nhako-beam.log"),
            old: dir.join("nhako-beam.1.log"),
            open: Mutex::new(None),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A writer for the app's logging (`tracing_subscriber::fmt::layer().with_writer(...)`).
    pub fn writer(self: &Arc<Self>) -> Writer {
        Writer(self.clone())
    }

    fn append(&self, buf: &[u8]) -> io::Result<()> {
        let mut open = self.open.lock().unwrap_or_else(|e| e.into_inner());
        let full = |size: u64| size > 0 && size + buf.len() as u64 > MAX_BYTES;
        if open.as_ref().is_some_and(|(_, size)| full(*size)) {
            *open = None;
            let _ = fs::rename(&self.path, &self.old);
        }
        if open.is_none() {
            if let Some(dir) = self.path.parent() {
                fs::create_dir_all(dir)?;
            }
            // A file left over from an earlier run may already be full.
            if full(fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0)) {
                let _ = fs::rename(&self.path, &self.old);
            }
            let f = OpenOptions::new().create(true).append(true).open(&self.path)?;
            let size = f.metadata()?.len();
            *open = Some((f, size));
        }
        let (f, size) = open.as_mut().expect("opened above");
        f.write_all(buf)?;
        *size += buf.len() as u64;
        Ok(())
    }

    /// The last `max` bytes of logs (older file first), starting at a whole line.
    pub fn tail(&self, max: usize) -> String {
        let _guard = self.open.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = Vec::new();
        let current = read_tail(&self.path, max);
        if current.len() < max {
            out.extend(read_tail(&self.old, max - current.len()));
        }
        out.extend(current);
        let text = String::from_utf8_lossy(&out).into_owned();
        // Drop the first line if the cut landed in the middle of it.
        match text.find('\n') {
            Some(i) if out.len() >= max => text[i + 1..].to_string(),
            _ => text,
        }
    }
}

fn read_tail(path: &Path, max: usize) -> Vec<u8> {
    let Ok(mut f) = File::open(path) else { return vec![] };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(max as u64);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return vec![];
    }
    let mut buf = Vec::new();
    let _ = f.take(max as u64).read_to_end(&mut buf);
    buf
}

pub struct Writer(Arc<LogFile>);

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.append(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_newest_lines_and_one_older_file() {
        let dir = std::env::temp_dir().join(format!("nhako-log-{}", rand::random::<u64>()));
        let log = Arc::new(LogFile::new(&dir));
        let line = "x".repeat(99) + "\n";
        for _ in 0..(3 * MAX_BYTES as usize / line.len()) {
            log.writer().write_all(line.as_bytes()).unwrap();
        }
        let size = |p: &Path| fs::metadata(p).map(|m| m.len()).unwrap_or(0);
        assert!(size(&log.path) <= MAX_BYTES);
        assert!(size(&log.old) <= MAX_BYTES && size(&log.old) > 0);
        writeln!(log.writer(), "the last line").unwrap();
        let tail = log.tail(1000);
        assert!(tail.len() <= 1000);
        assert!(tail.ends_with("the last line\n"));
        assert!(tail.starts_with('x') && tail.lines().next().unwrap().len() == 99, "starts at a whole line");
        let _ = fs::remove_dir_all(&dir);
    }
}
