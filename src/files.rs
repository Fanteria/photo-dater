use super::{file::File, files_interval::FilesInterval};
use crate::file::{ByCreatedDate, ByPath};
use anyhow::{anyhow, Result};
use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

/// Type alias for a collection of renamed files with their new paths.
pub type RenamedFiles<'a> = Vec<RenamedFile<'a>>;

/// Represents a file and its proposed new path for rename/move operations.
/// 
/// This structure pairs an original file reference with a new filesystem path.
#[derive(Debug, PartialEq, Eq)]
pub struct RenamedFile<'a>(pub &'a File, pub PathBuf);

/// A collection of files that provides various operations for file management and organization.
/// 
/// This struct wraps a `Vec<File>` and provides methods for reading files from directories,
/// grouping files by date, and organizing file operations.
#[derive(Debug)]
pub struct Files(Vec<File>);

impl Files {
    /// Creates a new Files collection from a vector of files.
    #[allow(dead_code)]
    pub fn new(files: Vec<File>) -> Self {
        Self(files)
    }

    /// Recursively reads all files from the specified directory path.
    /// 
    /// This method traverses the directory tree starting from the given path,
    /// collecting all files found in subdirectories. Files without EXIF data or
    /// creation dates are skipped.
    /// 
    /// # Arguments
    /// 
    /// * `path` - A path-like object that references the directory to read from
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The specified path cannot be read
    /// - File system permissions prevent access to files or directories
    /// - I/O errors occur during directory traversal
    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
        /// Recursive helper function to read files from a directory.
        fn read_dir(path: impl AsRef<Path>) -> Result<Vec<File>> {
            fs::read_dir(path.as_ref())?;
            Ok(fs::read_dir(path.as_ref())?
                .collect::<io::Result<Vec<_>>>()?
                .into_iter()
                .map(|e| e.path())
                .map(|p| -> Result<Vec<File>> {
                    if p.is_file() {
                        Ok(File::read(p)?.map(|f| vec![f]).unwrap_or_default())
                    } else if p.is_dir() {
                        read_dir(p)
                    } else {
                        Ok(vec![])
                    }
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>())
        }

        Ok(Self(read_dir(path)?))
    }

    /// This generic method allows sorting files by any ordering wrapper type
    /// that can be constructed from a file reference and implements `Ord`.
    /// 
    /// # Type Parameters
    /// 
    /// * `T` - The ordering wrapper type (e.g., `ByPath<&File>`, `ByCreatedDate<&File>`)
    /// 
    /// # Returns
    /// 
    /// A vector of file references sorted according to the specified ordering criterion.
    pub fn get_sorted<'a, T>(&'a self) -> Vec<&'a File>
    where
        T: Deref<Target = &'a File> + From<&'a File> + Ord,
    {
        let mut files: Vec<_> = self.iter().map(T::from).collect();
        files.sort();
        files.into_iter().map(|f| *f).collect()
    }

    /// Calculates the time interval spanning from the oldest to the newest file.
    /// Returns `None` if the collection is empty.
    pub fn interval(&self) -> Option<FilesInterval> {
        match (
            self.iter().map(ByCreatedDate).min(),
            self.iter().map(ByCreatedDate).max(),
        ) {
            (Some(from), Some(to)) => Some(FilesInterval {
                from: from.created,
                to: to.created,
            }),
            _ => None,
        }
    }

    /// This method creates a list of rename operations that would give all files
    /// sequential names with the specified base name. Files are sorted by path
    /// before numbering to ensure consistent ordering.
    /// 
    /// # Arguments
    /// 
    /// * `name` - The base name to use for renaming files
    /// 
    /// # Errors
    /// 
    /// Returns an error if file extensions contain non-UTF-8 characters.
    /// 
    /// # Examples
    /// 
    /// For files "b.jpg", "a.png", "c" with base name "photo":
    /// - "a.png" → "photo 0001.png"  
    /// - "b.jpg" → "photo 0002.jpg"
    /// - "c" → "photo 0003"
    #[allow(dead_code)]
    pub fn rename_files(&self, name: &str) -> Result<RenamedFiles> {
        self.get_sorted::<ByPath<&File>>()
            .into_iter()
            .enumerate()
            .map(|(i, file)| (i + 1, file))
            .map(|(i, file)| {
                let new_path = file.path.with_file_name(
                    file.path
                        .extension()
                        .map(|s| s.to_str().ok_or(anyhow!("Non UTF-8 file suffix.")))
                        .transpose()?
                        .map(|s| format!("{name} {i:04}.{s}"))
                        .unwrap_or(format!("{name} {i:04}")),
                );
                Ok(RenamedFile(file, new_path))
            })
            .collect()
    }

    /// Groups files by their creation date, with each group containing files from the same day.
    /// 
    /// Files are sorted by creation date and then grouped into vectors where each
    /// vector contains all files created on the same calendar day.
    pub fn group_by_days(&self) -> Vec<Vec<&File>> {
        let files = self.get_sorted::<ByCreatedDate<&File>>();
        let first_created = match files.first() {
            Some(file) => file.created.date(),
            None => return Vec::new(),
        };
        let (_, last_group, mut ret) = files.into_iter().fold(
            (first_created, Vec::new(), Vec::new()),
            |(mut last_created, mut group, mut acc), file| {
                let created = file.created.date();
                if last_created != created {
                    last_created = created;
                    acc.push(group);
                    group = Vec::new();
                }
                group.push(file);
                (last_created, group, acc)
            },
        );
        ret.push(last_group);
        ret
    }

    /// This method groups files by their creation date and generates new paths
    /// where each file would be moved to a subdirectory named after its creation date
    /// (formatted as "YYYY-MM-DD") within the same parent directory.
    /// 
    /// # Returns
    /// 
    /// A vector of vectors, where each inner vector represents a day's worth of files
    /// and contains `RenamedFile` instances with original file references and new paths.
    /// Files that cannot generate valid new paths (e.g., files without parent directories
    /// or file names) are filtered out.
    /// 
    /// # Examples
    /// 
    /// For a file "/photos/IMG_001.jpg" created on 2025-05-01:
    /// - New path would be "/photos/2025-05-01/IMG_001.jpg"
    pub fn move_by_days(&self) -> Vec<RenamedFiles> {
        self.group_by_days()
            .into_iter()
            .map(|group| {
                group
                    .into_iter()
                    .filter_map(|file| {
                        file.path
                            .parent()
                            .map(|parent| parent.join(file.created.format("%Y-%m-%d").to_string()))
                            .and_then(|path| Some(path.join(file.path.file_name()?)))
                            .map(|new_path| RenamedFile(file, new_path))
                    })
                    .collect()
            })
            .collect()
    }
}

impl Deref for Files {
    type Target = Vec<File>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Files {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::NaiveDateTime;

    use super::*;

    fn testing_files() -> [File; 3] {
        [
            File {
                path: PathBuf::from("./1.jpg"),
                created: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap(),
            },
            File {
                path: PathBuf::from("./2.png"),
                created: NaiveDateTime::from_str("2025-05-01T14:15:16").unwrap(),
            },
            File {
                path: PathBuf::from("./3"),
                created: NaiveDateTime::from_str("2025-05-03T12:13:14").unwrap(),
            },
        ]
    }

    #[test]
    fn interval() {
        let [file1, file2, file3] = testing_files();

        let files = Files(vec![]);
        assert_eq!(files.interval(), None);

        let files = Files([&file1].into_iter().cloned().collect());
        assert_eq!(
            files.interval(),
            Some(FilesInterval {
                from: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap(),
                to: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap()
            })
        );

        let files = Files([&file1, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.interval(),
            Some(FilesInterval {
                from: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap(),
                to: NaiveDateTime::from_str("2025-05-01T14:15:16").unwrap()
            })
        );

        let files = Files([&file1, &file3].into_iter().cloned().collect());
        assert_eq!(
            files.interval(),
            Some(FilesInterval {
                from: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap(),
                to: NaiveDateTime::from_str("2025-05-03T12:13:14").unwrap()
            })
        );

        let files = Files([&file1, &file3, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.interval(),
            Some(FilesInterval {
                from: NaiveDateTime::from_str("2025-05-01T12:13:14").unwrap(),
                to: NaiveDateTime::from_str("2025-05-03T12:13:14").unwrap()
            })
        );
    }

    #[test]
    fn rename_files() -> Result<()> {
        let [file1, file2, file3] = testing_files();

        let files = Files(vec![]);
        assert_eq!(files.rename_files("new name")?, vec![]);

        let files = Files([&file1].into_iter().cloned().collect());
        assert_eq!(
            files.rename_files("new_name")?,
            vec![RenamedFile(&file1, PathBuf::from("./new_name 0001.jpg"))]
        );

        let files = Files([&file1, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.rename_files("new_name")?,
            vec![
                RenamedFile(&file1, PathBuf::from("./new_name 0001.jpg")),
                RenamedFile(&file2, PathBuf::from("./new_name 0002.png"))
            ]
        );

        let files = Files([&file1, &file3].into_iter().cloned().collect());
        assert_eq!(
            files.rename_files("new_name")?,
            vec![
                RenamedFile(&file1, PathBuf::from("./new_name 0001.jpg")),
                RenamedFile(&file3, PathBuf::from("./new_name 0002")),
            ]
        );

        let files = Files([&file1, &file3, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.rename_files("new_name")?,
            vec![
                RenamedFile(&file1, PathBuf::from("./new_name 0001.jpg")),
                RenamedFile(&file2, PathBuf::from("./new_name 0002.png")),
                RenamedFile(&file3, PathBuf::from("./new_name 0003")),
            ]
        );

        Ok(())
    }

    #[test]
    fn move_by_days() {
        let [file1, file2, file3] = testing_files();

        let files = Files(vec![]);
        assert_eq!(files.move_by_days(), Vec::<RenamedFiles>::new());

        let files = Files([&file1].into_iter().cloned().collect());
        assert_eq!(
            files.move_by_days(),
            vec![vec![RenamedFile(
                &file1,
                PathBuf::from("./2025-05-01/1.jpg")
            )]]
        );

        let files = Files([&file1, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.move_by_days(),
            vec![vec![
                RenamedFile(&file1, PathBuf::from("./2025-05-01/1.jpg")),
                RenamedFile(&file2, PathBuf::from("./2025-05-01/2.png"))
            ]]
        );

        let files = Files([&file1, &file3].into_iter().cloned().collect());
        assert_eq!(
            files.move_by_days(),
            vec![
                vec![RenamedFile(&file1, PathBuf::from("./2025-05-01/1.jpg"))],
                vec![RenamedFile(&file3, PathBuf::from("./2025-05-03/3"))],
            ]
        );

        let files = Files([&file1, &file3, &file2].into_iter().cloned().collect());
        assert_eq!(
            files.move_by_days(),
            vec![
                vec![
                    RenamedFile(&file1, PathBuf::from("./2025-05-01/1.jpg")),
                    RenamedFile(&file2, PathBuf::from("./2025-05-01/2.png"))
                ],
                vec![RenamedFile(&file3, PathBuf::from("./2025-05-03/3"))],
            ]
        );
    }
}
