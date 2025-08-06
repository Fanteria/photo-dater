use std::path::PathBuf;

use crate::files::Files;
use crate::files_interval::FilesInterval;

use anyhow::{anyhow, Result};

pub enum NameStatus {
    Valid,
    Invalid,
    None,
}

pub struct Directory {
    pub directory: PathBuf,
    files: Files,
}

impl Directory {
    pub fn try_from(directory: PathBuf) -> Result<Self> {
        if !directory.is_dir() {
            return Err(anyhow!("{:?} is not directory", directory));
        }
        Ok(Directory {
            files: Files::read(&directory)?,
            directory,
        })
    }

    pub fn name(&self) -> Option<&str> {
        self.directory.file_name().and_then(|s| s.to_str())
    }

    fn get_status(interval: &FilesInterval, name: &str) -> NameStatus {
        match FilesInterval::try_from_name(name) {
            Some(name_interval) if name_interval == *interval => NameStatus::Valid,
            Some(_) => NameStatus::Invalid,
            None => NameStatus::None,
        }
    }

    pub fn name_status(&self) -> Result<NameStatus> {
        let name = self.name().ok_or(anyhow!("Invalid directory name"))?;
        let interval = self
            .files
            .interval()
            .ok_or(anyhow!("Does not get interval"))?;
        Ok(Self::get_status(&interval, name))
    }

    pub fn rename(&self, max_interval: u32) -> Result<(NameStatus, PathBuf)> {
        let interval = self
            .files
            .interval()
            .ok_or(anyhow!("Does not get interval"))?;
        let delta = interval.delta();
        if delta.abs().num_days() > max_interval.into() {
            return Err(anyhow!(
                "Interval from {} to {} is too large ({} days)",
                interval.from,
                interval.to,
                delta.num_days()
            ));
        }
        let old_name = self
            .directory
            .file_name()
            .ok_or(anyhow!("Cannot get filename from {:?}", self.directory))?
            .to_str()
            .ok_or(anyhow!(
                "File name {:?} is not UTF-8 valid string",
                self.directory
            ))?;
        let new_name = format!("{} {}", interval, old_name);
        let new_path = self.directory.with_file_name(new_name);
        Ok((Self::get_status(&interval, old_name), new_path))
    }

    pub fn get_files(&self) -> &Files {
        &self.files
    }

    pub fn get_mut_files(&mut self) -> &mut Files {
        &mut self.files
    }
}
