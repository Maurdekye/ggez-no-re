use std::{
    fs::{OpenOptions, create_dir_all},
    io,
    path::PathBuf,
};

use log::error;
use serde::Serialize;

#[derive(Clone)]
pub struct CsvRecorder {
    base_path: PathBuf,
}

impl CsvRecorder {
    pub fn new(base_path: impl Into<PathBuf>) -> CsvRecorder {
        CsvRecorder {
            base_path: base_path.into(),
        }
    }

    pub fn record(&self, subpath: impl Into<PathBuf>, record: impl Serialize) {
        let _ = self
            .record_inner(subpath, record)
            .map_err(|e| error!("{e}"));
    }

    pub fn record_inner(
        &self,
        subpath: impl Into<PathBuf>,
        record: impl Serialize,
    ) -> Result<(), io::Error> {
        let mut path = self.base_path.clone();
        path.push(subpath.into());
        if !path.ends_with(".csv") {
            path.add_extension("csv");
        }
        let _ = create_dir_all(&path.parent().unwrap());
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);
        writer.serialize(record)?;
        writer.flush()?;
        Ok(())
    }
}
