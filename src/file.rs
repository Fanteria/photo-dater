use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

#[derive(Debug, Clone)]
pub struct File {
    pub path: PathBuf,
    pub created: NaiveDateTime,
}

impl File {
    // If does not contain exif info or does not contain DateTimeOriginal skip file
    pub fn read(path: PathBuf) -> Result<Option<Self>> {
        let file = std::fs::File::open(&path)?;
        let mut bufreader = std::io::BufReader::new(&file);
        exif::Reader::new()
            .read_from_container(&mut bufreader)
            .ok()
            .and_then(|exif| {
                exif.fields()
                    .find(|f| f.tag == exif::Tag::DateTimeOriginal)
                    .map(|f| {
                        let date_str = f.display_value().with_unit(&exif).to_string();
                        let created = NaiveDateTime::parse_from_str(&date_str, "%Y-%m-%d %H:%M:%S")
                            .or_else(|_| {
                                NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d %H:%M:%S")
                            })
                            .map_err(|_| anyhow!("Failed to parse {path:?} date: {date_str}"))?;
                        Ok(File { path, created })
                    })
            })
            .transpose()
    }
}

/// Add ordering by path.
pub struct ByPath<T>(pub T);

impl<T> Ord for ByPath<T>
where
    T: Deref<Target = File>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.path.cmp(&other.0.path)
    }
}
impl<T> PartialOrd for ByPath<T>
where
    T: Deref<Target = File>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> PartialEq for ByPath<T>
where
    T: Deref<Target = File>,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.path == other.0.path
    }
}
impl<T> Eq for ByPath<T> where T: Deref<Target = File> {}

impl<T> DerefMut for ByPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T> Deref for ByPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Add ordering by created date.
pub struct ByCreatedDate<T>(pub T);

impl<T> Ord for ByCreatedDate<T>
where
    T: Deref<Target = File>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.created.cmp(&other.0.created)
    }
}
impl<T> PartialOrd for ByCreatedDate<T>
where
    T: Deref<Target = File>,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> PartialEq for ByCreatedDate<T>
where
    T: Deref<Target = File>,
{
    fn eq(&self, other: &Self) -> bool {
        self.0.created == other.0.created
    }
}
impl<T> Eq for ByCreatedDate<T> where T: Deref<Target = File> {}

impl<T> DerefMut for ByCreatedDate<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T> Deref for ByCreatedDate<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
