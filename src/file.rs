use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use std::{
    cmp::Ordering,
    io::{Read, Seek},
    ops::{Deref, DerefMut},
    path::PathBuf,
};

/// Represents a photo file with its filesystem path and creation date.
///
/// This struct encapsulates a file's location and the creation timestamp
/// extracted from its EXIF metadata. Only files with valid EXIF creation
/// dates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: PathBuf,
    pub created: NaiveDateTime,
}

impl File {
    /// This method attempts to parse EXIF metadata from the provided reader
    /// and extract the DateTimeOriginal field.
    /// 
    /// # Arguments
    /// 
    /// * `reader` - A reader that implements `Read + Seek` for accessing file data
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(Some(NaiveDateTime))` if EXIF data is found and contains a valid
    /// creation date, `Ok(None)` if no EXIF data or creation date is found, or an
    /// error if the date string cannot be parsed.
    /// 
    /// # Supported Date Formats
    /// 
    /// - `%Y-%m-%d %H:%M:%S` (e.g., "2025-05-01 14:30:25")
    /// - `%Y:%m:%d %H:%M:%S` (e.g., "2025:05:01 14:30:25")
    fn read_time<R>(reader: R) -> Result<Option<NaiveDateTime>>
    where
        R: Read + Seek,
    {
        let mut bufreader = std::io::BufReader::new(reader);
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
                            .context(format!("Failed to parse date: {date_str}"))?;
                        Ok(created)
                    })
            })
            .transpose()
    }

    /// This method opens the file at the specified path and attempts to extract
    /// the creation date from its EXIF metadata. Files without EXIF data or
    /// without a DateTimeOriginal field are skipped (return None).
    /// 
    /// # Arguments
    /// 
    /// * `path` - Path to the file to read
    /// 
    /// # Returns
    /// 
    /// Returns `Ok(Some(File))` if the file contains valid EXIF creation date,
    /// `Ok(None)` if the file has no EXIF data or creation date, or an error
    /// if the file cannot be read or the date cannot be parsed.
    /// 
    /// # Errors
    /// 
    /// This function will return an error if:
    /// - The file cannot be opened (permissions, not found, etc.)
    /// - The EXIF date string is present but cannot be parsed
    /// - I/O errors occur while reading the file
    pub fn read(path: PathBuf) -> Result<Option<Self>> {
        let file = std::fs::File::open(&path)?;
        Self::read_time(file)
            .context(format!("Path: {path:?}"))
            .map(|opt_time| opt_time.map(|created| File { path, created }))
    }
}

/// Wrapper type that adds path-based ordering to any type that dereferences to File.
/// 
/// This struct allows sorting collections of files (or file references) by their
/// filesystem paths in lexicographical order.
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

impl<'a, T> From<&'a T> for ByPath<&'a T> {
    fn from(value: &'a T) -> Self {
        ByPath::<&'a T>(value)
    }
}

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

/// Wrapper type that adds creation date-based ordering to any type that dereferences to File.
/// 
/// This struct allows sorting collections of files (or file references) by their
/// creation timestamps in chronological order.
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

impl<'a, T> From<&'a T> for ByCreatedDate<&'a T> {
    fn from(value: &'a T) -> Self {
        ByCreatedDate::<&'a T>(value)
    }
}

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

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveTime};

    use super::*;

    /// Helper function to create NaiveDateTime instances for testing.
    fn datetime(y: i32, m: u32, d: u32, hour: u32, min: u32, sec: u32) -> NaiveDateTime {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(y, m, d).unwrap(),
            NaiveTime::from_hms_opt(hour, min, sec).unwrap(),
        )
    }

    #[test]
    fn cmp_by_path() {
        let created = datetime(2025, 5, 1, 10, 11, 12);
        let mut first_file = File {
            path: PathBuf::from("./some/path/1.jpg"),
            created,
        };
        let mut second_file = File {
            path: PathBuf::from("./some/path/2.jpg"),
            created,
        };
        assert_eq!(
            ByPath::<&File>(&first_file).cmp(&ByPath::<&File>(&second_file)),
            Ordering::Less
        );

        assert_eq!(
            ByPath::<&File>(&second_file).cmp(&ByPath::<&File>(&first_file)),
            Ordering::Greater
        );

        assert_eq!(
            ByPath::<&File>(&first_file).cmp(&ByPath::<&File>(&first_file)),
            Ordering::Equal
        );

        assert_eq!(
            ByPath::<&mut File>(&mut first_file).cmp(&ByPath::<&mut File>(&mut second_file)),
            Ordering::Less
        );

        let min = ByPath::<&File>(&first_file).min(ByPath::<&File>(&second_file));
        assert_eq!(&min.path, &first_file.path);

        {
            let mut min =
                ByPath::<&mut File>(&mut first_file).min(ByPath::<&mut File>(&mut second_file));
            min.path = PathBuf::from("./some/path/3.jpg");
        }
        assert_eq!(first_file.path, PathBuf::from("./some/path/3.jpg"));
    }

    #[test]
    fn cmp_by_date() {
        let mut first_file = File {
            path: PathBuf::new(),
            created: datetime(2025, 5, 1, 10, 11, 12),
        };
        let mut second_file = File {
            path: PathBuf::new(),
            created: datetime(2025, 5, 1, 10, 11, 13),
        };

        assert_eq!(
            ByCreatedDate::<&File>(&first_file).cmp(&ByCreatedDate::<&File>(&second_file)),
            Ordering::Less
        );

        assert_eq!(
            ByCreatedDate::<&mut File>(&mut first_file)
                .cmp(&ByCreatedDate::<&mut File>(&mut second_file)),
            Ordering::Less
        );

        assert_eq!(
            ByCreatedDate::<&File>(&second_file).cmp(&ByCreatedDate::<&File>(&first_file)),
            Ordering::Greater
        );

        assert_eq!(
            ByCreatedDate::<&File>(&first_file).cmp(&ByCreatedDate::<&File>(&first_file)),
            Ordering::Equal
        );

        let min = ByCreatedDate::<&File>(&first_file).min(ByCreatedDate::<&File>(&second_file));
        assert_eq!(&min.created, &first_file.created);

        {
            let mut min = ByCreatedDate::<&mut File>(&mut first_file)
                .min(ByCreatedDate::<&mut File>(&mut second_file));
            min.created = datetime(2025, 5, 1, 10, 11, 13);
        }
        assert_eq!(first_file.created, datetime(2025, 5, 1, 10, 11, 13));
    }
}
