use crate::files::Files;
use crate::files_interval::FilesInterval;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NameStatus {
    Valid,
    Invalid,
    SuperSet,
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

    pub fn name(&self) -> Result<&str> {
        self.directory
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid directory name")
    }

    fn interval(&self) -> Result<FilesInterval> {
        self.files.interval().context("Does not get interval")
    }

    fn get_status(interval: &FilesInterval, name: &str) -> NameStatus {
        match FilesInterval::try_from_name(name) {
            Some(FilesInterval { from, to })
                if from.date() == interval.from.date() && to.date() == interval.to.date() =>
            {
                NameStatus::Valid
            }
            Some(FilesInterval { from, to }) if from <= interval.from && to >= interval.to => {
                NameStatus::SuperSet
            }
            Some(_) => NameStatus::Invalid,
            None => NameStatus::None,
        }
    }

    pub fn name_status(&self) -> Result<NameStatus> {
        Ok(Self::get_status(&self.interval()?, self.name()?))
    }

    pub fn rename(&self, max_interval: u32) -> Result<(NameStatus, PathBuf)> {
        let interval = self.interval()?;
        let delta = self.interval()?.delta();
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
        let status = Self::get_status(&interval, old_name);
        Ok((
            status,
            match status {
                NameStatus::Valid => self.directory.clone(),
                // TODO how to solve invalid dates???
                NameStatus::Invalid => self
                    .directory
                    .with_file_name(format!("{} {}", interval, old_name)),
                NameStatus::SuperSet => self.directory.clone(),
                NameStatus::None => self
                    .directory
                    .with_file_name(format!("{} {}", interval, old_name)),
            },
        ))
    }

    pub fn get_files(&self) -> &Files {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDateTime;

    use crate::file::File;

    use super::*;

    fn test_files() -> [File; 2] {
        [
            File {
                path: PathBuf::new(),
                created: NaiveDateTime::from_str("2025-05-01T12:00:00").unwrap(),
            },
            File {
                path: PathBuf::new(),
                created: NaiveDateTime::from_str("2025-05-03T12:00:00").unwrap(),
            },
        ]
    }

    #[test]
    fn name_status() {
        let [file1, file2] = test_files();

        // Single file
        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::Valid);

        let dir = Directory {
            directory: PathBuf::from("./2025-05-02 dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::Invalid);

        let dir = Directory {
            directory: PathBuf::from("dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::None);

        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::Valid);

        // Multiple files
        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 - 03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::Valid);

        let dir = Directory {
            directory: PathBuf::from("./2026-05-01 - 03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::Invalid);

        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 - 04 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::SuperSet);

        let dir = Directory {
            directory: PathBuf::from("./2025-04-30 - 05-03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::SuperSet);

        let dir = Directory {
            directory: PathBuf::from("./2025-04-30 - 2026-01-01 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(dir.name_status().unwrap(), NameStatus::SuperSet);
    }

    #[test]
    fn rename() {
        let [file1, file2] = test_files();

        // Single file
        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(0).unwrap(),
            (NameStatus::Valid, PathBuf::from("./2025-05-01 dir name"))
        );

        let dir = Directory {
            directory: PathBuf::from("./2025-05-03 dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(0).unwrap(),
            (
                NameStatus::Invalid,
                PathBuf::from("./2025-05-01 2025-05-03 dir name")
            )
        );

        let dir = Directory {
            directory: PathBuf::from("./dir name"),
            files: Files::new([&file1].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(0).unwrap(),
            (NameStatus::None, PathBuf::from("./2025-05-01 dir name"))
        );

        // Multiple files
        let dir = Directory {
            directory: PathBuf::from("./Too long interval"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert!(dir.rename(0).is_err());

        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 - 03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(2).unwrap(),
            (
                NameStatus::Valid,
                PathBuf::from("./2025-05-01 - 03 dir name")
            )
        );

        let dir = Directory {
            directory: PathBuf::from("./2026-05-01 - 03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(2).unwrap(),
            (
                NameStatus::Invalid,
                PathBuf::from("./2025-05-01 - 03 2026-05-01 - 03 dir name")
            )
        );

        let dir = Directory {
            directory: PathBuf::from("./2025-05-01 - 04 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(2).unwrap(),
            (
                NameStatus::SuperSet,
                PathBuf::from("./2025-05-01 - 04 dir name")
            )
        );

        let dir = Directory {
            directory: PathBuf::from("./2025-04-30 - 05-03 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(2).unwrap(),
            (
                NameStatus::SuperSet,
                PathBuf::from("./2025-04-30 - 05-03 dir name")
            )
        );

        let dir = Directory {
            directory: PathBuf::from("./2025-04-30 - 2026-01-01 dir name"),
            files: Files::new([&file1, &file2].into_iter().cloned().collect()),
        };
        assert_eq!(
            dir.rename(2).unwrap(),
            (
                NameStatus::SuperSet,
                PathBuf::from("./2025-04-30 - 2026-01-01 dir name")
            )
        );
    }
}
