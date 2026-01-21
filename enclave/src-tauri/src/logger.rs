use std::{fs::{File, OpenOptions, create_dir_all}, io::{BufWriter, Write}, path::Path, sync::Mutex};

use log::{LevelFilter, Record};

pub struct Logger {
    level: LevelFilter,
    writer: Mutex<BufWriter<File>>,
}

impl Logger {
    pub fn new(path: &str, level: LevelFilter) -> std::io::Result<Self> {
        if let Some(parent) = Path::new(path).parent() {
            create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;

        Ok(Self {
            level,
            writer: Mutex::new(BufWriter::new(file)),
        })
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let mut writer = match self.writer.lock() {
            Ok(w) => w,
            Err(poisoned) => poisoned.into_inner(),
        };

        
        println!("Called");
        println!("Writing: [{}] {}", record.level(), record.args());

        let _ = writeln!(
            writer,
            "[{}] {}",
            record.level(),
            record.args()
        );
        let _ = writer.flush();
    }

    fn flush(&self) {
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.flush();
        }
    }
}