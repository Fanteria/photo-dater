use super::{file::File, files_interval::FilesInterval};
use crate::file::ByCreatedDate;
use anyhow::Result;
use chrono::NaiveDateTime;
use std::{
    fs, io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Files(Vec<File>);

impl Files {
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

    pub fn rename_files(&self, _name: &str) -> Vec<(&File, PathBuf)> {
        // let mut files: Vec<_> = self.iter_mut().map(ByPath::<&mut File>).collect();
        // files.sort();
        // files.iter_mut().enumerate().for_each(|(i, file)| {
        //     file.path.set_file_name(format!("{name} {i:04}"));
        // });
        vec![]
    }

    pub fn group_by_days(&self) -> Vec<Vec<&File>> {
        let mut files: Vec<_> = self.iter().map(ByCreatedDate::<&File>).collect();
        files.sort();
        let files: Vec<_> = files.into_iter().map(|f| *f).collect();

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

    pub fn move_by_days(&self) -> Vec<Vec<(&File, PathBuf)>> {
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
                            .map(|new_path| (file, new_path))
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
