use crate::files::Files;
use crate::files_interval::FilesInterval;
use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

/// Status of a directory's name relative to its file contents' date range.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NameStatus {
    /// Directory name exactly matches the date range of contained files
    Valid,
    /// Directory name contains date information but doesn't match file dates
    Invalid,
    /// Directory name represents a broader date range than the actual files
    SuperSet,
    /// Directory name contains no date information
    None,
}

/// Represents a directory containing photo files with date-based analysis capabilities.
/// 
/// This structure encapsulates a directory path and its contained files, providing
/// methods to analyze the relationship between the directory's name and the
/// date range of its contents.
pub struct Directory {
    /// The path to the directory
    pub directory: PathBuf,
    /// Collection of files found in the directory (recursively)
    files: Files,
}

impl Directory {
    /// Creates a new Directory instance from the given path.
    /// 
    /// This constructor validates that the path is actually a directory and
    /// recursively reads all files contained within it.
    /// 
    /// # Arguments
    /// 
    /// * `directory` - Path to the directory to analyze
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The provided path is not a directory
    /// - The directory cannot be read due to permissions or I/O errors
    /// - Files within the directory cannot be processed
    pub fn try_from(directory: PathBuf) -> Result<Self> {
        if !directory.is_dir() {
            return Err(anyhow!("{:?} is not directory", directory));
        }
        Ok(Directory {
            files: Files::read(&directory)?,
            directory,
        })
    }

    /// Extracts the directory name as a string slice.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the directory path has no filename component
    /// or if the filename is not valid UTF-8.
    pub fn name(&self) -> Result<&str> {
        self.directory
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid directory name")
    }

    /// Calculates the date interval spanned by files in this directory.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the directory contains no files with valid dates.
    fn interval(&self) -> Result<FilesInterval> {
        self.files.interval().context("Does not get interval")
    }

    /// This method compares a directory name against a file date interval to
    /// determine if the name appropriately represents the content.
    /// 
    /// # Arguments
    /// 
    /// * `interval` - The actual date range of files in the directory
    /// * `name` - The directory name to evaluate
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

    /// Evaluates the current directory name against its file contents.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the directory name cannot be extracted or if
    /// the file date interval cannot be determined.
    pub fn name_status(&self) -> Result<NameStatus> {
        Ok(Self::get_status(&self.interval()?, self.name()?))
    }

    /// This method analyzes the current directory name and file date range to
    /// suggest an appropriate new name that reflects the actual content dates.
    /// 
    /// # Arguments
    /// 
    /// * `max_interval` - Maximum allowed interval in days between oldest and newest files
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The date interval exceeds the maximum allowed interval
    /// - The directory name cannot be extracted or is not valid UTF-8
    /// - The file date interval cannot be determined
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

    /// Provides read-only access to the files contained in this directory.
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

    /// Helper function to create test files with specific creation dates.
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
