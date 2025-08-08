use anyhow::{anyhow, Result};
use std::{fmt::Display, str::FromStr};

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};

#[derive(Debug, PartialEq, Eq)]
pub struct FilesInterval {
    pub from: NaiveDateTime,
    pub to: NaiveDateTime,
}

const SEPARATOR: &str = " - ";

impl FilesInterval {
    pub fn try_from_name(name: &str) -> Option<Self> {
        let (from, to) = name
            // Try if from and to differs.
            .split_once(SEPARATOR)
            .and_then(|(from, name)| name.split_once(' ').map(|(to, _name)| (from, to)))
            .and_then(|(from, to)| {
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
                Some((from, to))
            })
            // From and to are same day.
            .or_else(|| {
                let (from_str, _name) = name.split_once(' ')?;
                let from = NaiveDate::from_str(from_str).ok()?;
                Some((from, from))
            })?;
        Self::from_date(from, to).ok()
    }

    pub fn delta(&self) -> TimeDelta {
        self.to - self.from
    }

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
