use super::{file::File, files_interval::FilesInterval};
use crate::file::{ByCreatedDate, ByPath};
use anyhow::{anyhow, Result};
use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

pub type RenamedFiles<'a> = Vec<RenamedFile<'a>>;
#[derive(Debug, PartialEq, Eq)]
pub struct RenamedFile<'a>(pub &'a File, pub PathBuf);

#[derive(Debug)]
pub struct Files(Vec<File>);

impl Files {
    #[allow(dead_code)]
    pub fn new(files: Vec<File>) -> Self {
        Self(files)
    }

    pub fn read(path: impl AsRef<Path>) -> Result<Self> {
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

    pub fn get_sorted<'a, T>(&'a self) -> Vec<&'a File>
    where
        T: Deref<Target = &'a File> + From<&'a File> + Ord,
    {
        let mut files: Vec<_> = self.iter().map(T::from).collect();
        files.sort();
        files.into_iter().map(|f| *f).collect()
    }

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
