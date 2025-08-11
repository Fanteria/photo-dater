use anyhow::{anyhow, Result};
use std::{fmt::Display, str::FromStr};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};

/// Represents a time interval between creation date of first and last photo.
#[derive(Debug, PartialEq, Eq)]
pub struct FilesInterval {
    pub from: NaiveDateTime,
    pub to: NaiveDateTime,
}

const SEPARATOR: &str = " - ";

impl FilesInterval {
    /// This method recognizes various directory naming patterns that include date ranges and splits
    /// the input into a date interval and the remaining descriptive name portion.
    ///
    /// # Supported Formats
    ///
    /// - **Single date**: `"2025-05-01 My Photos"` -> May 1st only, remaining: "My Photos"
    /// - **Full range**: `"2025-05-01 - 2025-05-03 My Photos"` -> May 1st to 3rd 2025, remaining: "My Photos"
    /// - **Same year**: `"2025-05-01 - 05-03 My Photos"` -> May 1st to 3rd 2025, remaining: "My Photos"  
    /// - **Same month**: `"2025-05-01 - 03 My Photos"` -> May 1st to 3rd 2025, remaining: "My Photos"
    ///
    /// # Arguments
    ///
    /// * `name` - The directory name string to parse
    ///
    /// # Returns
    ///
    /// Returns `Some((FilesInterval, &str))` if a valid date pattern is found, where the tuple contains
    /// the parsed date interval and the remaining name portion after the date.
    /// Returns `None` if no recognizable date pattern exists.
    pub fn try_split(name: &str) -> Option<(Self, &str)> {
        let (from, to, name) = name
            // Try if from and to differs.
            .split_once(SEPARATOR)
            .and_then(|(from, name)| name.split_once(' ').map(|(to, name)| (from, to, name)))
            .and_then(|(from, to, name)| {
                let from = NaiveDate::from_str(from).ok()?;
                // Check if date is `yyyy-mm-dd`
                let to = NaiveDate::from_str(to)
                    .or_else(|_| {
                        // Check if date is `mm-dd`
                        NaiveDate::from_str(&format!("{:04}-{to}", from.year()))
                    })
                    .or_else(|_| {
                        // Check if date is `dd`
                        NaiveDate::from_str(&format!("{:04}-{:02}-{to}", from.year(), from.month()))
                    })
                    .ok()?;
                Some((from, to, name))
            })
            // From and to are same day.
            .or_else(|| {
                let (from_str, name) = name.split_once(' ')?;
                let from = NaiveDate::from_str(from_str).ok()?;
                Some((from, from, name))
            })?;
        Self::from_date(from, to)
            .ok()
            .map(|interval| (interval, name))
    }

    /// This method recognizes various directory naming patterns that include date ranges:
    ///
    /// # Supported Formats
    ///
    /// - **Single date**: `"2025-05-01 My Photos"` -> May 1st only
    /// - **Full range**: `"2025-05-01 - 2025-05-03 My Photos"` -> May 1st to 3rd, 2025
    /// - **Same year**: `"2025-05-01 - 05-03 My Photos"` -> May 1st to 3rd, 2025  
    /// - **Same month**: `"2025-05-01 - 03 My Photos"` -> May 1st to 3rd, 2025
    ///
    /// # Arguments
    ///
    /// * `name` - The directory name string to parse
    ///
    /// # Returns
    ///
    /// Returns `Some(FilesInterval)` if a valid date pattern is found,
    /// or `None` if no recognizable date pattern exists.
    pub fn try_from_name(name: &str) -> Option<Self> {
        Self::try_split(name).map(|(interval, _name)| interval)
    }

    /// Calculates the time duration of this interval.
    pub fn delta(&self) -> TimeDelta {
        self.to - self.from
    }

    /// Creates a FilesInterval from start and end dates.
    ///
    /// This method constructs a FilesInterval where the start time begins at
    /// the beginning of the `from` date (00:00:00) and the end time extends
    /// to the last second of the `to` date (23:59:59).
    ///
    /// # Arguments
    ///
    /// * `from` - The start date of the interval
    /// * `to` - The end date of the interval
    ///
    /// # Errors
    ///
    /// Returns an error if the `from` date is later than the `to` date.
    fn from_date(from: NaiveDate, to: NaiveDate) -> Result<Self> {
        if from > to {
            return Err(anyhow!("from date {from} is higher than to date {to}"));
        }
        Ok(Self {
            from: NaiveDateTime::new(from, NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            to: NaiveDateTime::new(to, NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
        })
    }
}

impl Display for FilesInterval {
    /// Formats the interval as a string suitable for directory names.
    ///
    /// This implementation uses intelligent formatting to create compact,
    /// readable date ranges:
    ///
    /// # Formatting Rules
    ///
    /// - **Single day**: `"2025-05-01"`
    /// - **Different years**: `"2025-05-01 - 2026-06-02"`
    /// - **Same year, different months**: `"2025-05-01 - 06-02"`  
    /// - **Same month**: `"2025-05-01 - 02"`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ret = self.from.format("%Y-%m-%d").to_string();
        if self.from.date() == self.to.date() {
            return f.write_str(&ret);
        }
        ret += SEPARATOR;
        if self.from.year() != self.to.year() {
            ret += &self.to.format("%Y-%m-%d").to_string();
        } else if self.from.month() != self.to.month() {
            ret += &self.to.format("%m-%d").to_string();
        } else {
            ret += &self.to.format("%d").to_string();
        }

        f.write_str(&ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_files_interval(from: (i32, u32, u32), to: Option<(i32, u32, u32)>) -> FilesInterval {
        let (fy, fm, fd) = from;
        let (ty, tm, td) = to.unwrap_or(from);
        FilesInterval {
            from: NaiveDateTime::new(
                NaiveDate::from_ymd_opt(fy, fm, fd).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            to: NaiveDateTime::new(
                NaiveDate::from_ymd_opt(ty, tm, td).unwrap(),
                NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            ),
        }
    }

    #[test]
    fn try_from_name() {
        // Basic usage
        assert_eq!(
            FilesInterval::try_from_name("Some name without any date"),
            None
        );

        assert_eq!(
            FilesInterval::try_from_name("2025-05-01 Some name"),
            Some(new_files_interval((2025, 5, 1), None)),
        );

        assert_eq!(
            FilesInterval::try_from_name("2025-05-01 - 2026-06-01 Some name"),
            Some(new_files_interval((2025, 5, 1), Some((2026, 6, 1)))),
        );

        assert_eq!(
            FilesInterval::try_from_name("2025-05-01 - 06-01 Some name"),
            Some(new_files_interval((2025, 5, 1), Some((2025, 6, 1)))),
        );

        // Edge cases
        assert_eq!(
            FilesInterval::try_from_name("06-01 Name start with number"),
            None
        );

        assert_eq!(
            FilesInterval::try_from_name("2025-05-01 - Name start with separator"),
            Some(new_files_interval((2025, 5, 1), None)),
        );

        assert_eq!(
            FilesInterval::try_from_name("2025-05-02 - 2025-05-01 - Interval is not possilbe"),
            None,
        );
    }

    #[test]
    fn delta() {
        assert_eq!(
            new_files_interval((2025, 5, 1), None).delta(),
            TimeDelta::seconds(23 * 60 * 60 + 59 * 60 + 59)
        );

        assert_eq!(
            new_files_interval((2025, 5, 1), Some((2025, 5, 5))).delta(),
            TimeDelta::seconds(23 * 60 * 60 + 59 * 60 + 59) + TimeDelta::days(4)
        );
    }

    #[test]
    fn to_string() {
        assert_eq!(
            &new_files_interval((2025, 5, 1), None).to_string(),
            "2025-05-01"
        );

        assert_eq!(
            &new_files_interval((2025, 5, 1), Some((2026, 6, 2))).to_string(),
            "2025-05-01 - 2026-06-02"
        );

        assert_eq!(
            &new_files_interval((2025, 5, 1), Some((2025, 6, 2))).to_string(),
            "2025-05-01 - 06-02"
        );

        assert_eq!(
            &new_files_interval((2025, 5, 1), Some((2025, 5, 2))).to_string(),
            "2025-05-01 - 02"
        );

        assert_eq!(
            &new_files_interval((2025, 5, 1), Some((2025, 6, 1))).to_string(),
            "2025-05-01 - 06-01"
        );

        assert_eq!(
            &new_files_interval((2025, 5, 1), Some((2026, 5, 1))).to_string(),
            "2025-05-01 - 2026-05-01"
        );
    }
}
