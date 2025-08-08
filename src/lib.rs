mod directory;
mod file;
mod files;
mod files_interval;

use anyhow::Result;
use clap::{Parser, Subcommand};
use file::File;
use std::{ffi::OsString, fs, io, path::PathBuf};

use crate::{directory::Directory, file::ByCreatedDate, files::RenamedFile};

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    /// TODO
    Status,
    /// TODO
    Rename {
        /// Maximal interval in days
        #[arg(default_value = "0")]
        max_interval: u32,
        #[arg(short, long)]
        dry_run: bool,
    },
    /// TODO
    List,
    /// TODO
    Interval,
    /// TODO
    Check {
        /// Maximal interval in days
        max_interval: u32,
    },
    /// TODO
    FilesRename {
        #[arg(short, long)]
        dry_run: bool,
        #[arg(short, long)]
        name: Option<String>,
    },
    /// TODO
    MoveByDays {
        #[arg(short, long)]
        dry_run: bool,
    },
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(short, long, default_value = ".")]
    directory: PathBuf,
}

pub fn run<I, T, WStd, WErr>(args: I, mut std: WStd, mut err: WErr) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
    WStd: io::Write,
    WErr: io::Write,
{
    let Cli { cmd, directory } = Cli::try_parse_from(args)?;
    let directory = Directory::try_from(directory)?;
    match cmd {
        Commands::Status => match directory.name_status() {
            Ok(directory::NameStatus::Valid) => writeln!(std, "Date is valid")?,
            Ok(directory::NameStatus::Invalid) => writeln!(std, "Date is set but is invalid")?,
            Ok(directory::NameStatus::SuperSet) => writeln!(std, "Date is set but is superset")?,
            Ok(directory::NameStatus::None) => writeln!(std, "Date is not set")?,
            Err(e) => writeln!(std, "Failed to get status '{}'", e)?,
        },
        Commands::Rename {
            max_interval,
            dry_run,
        } => {
            let (status, new_path) = directory.rename(max_interval)?;
            use directory::NameStatus as NS;
            match status {
                NS::Valid => writeln!(err, "Directory already have right date")?,
                NS::Invalid => writeln!(
                    err,
                    "Directory already have date, but it is not match content"
                )?,
                NS::SuperSet => writeln!(
                    err,
                    "Directories name is already super set of the right name"
                )?,
                NS::None => {
                    if !dry_run {
                        fs::rename(&directory.directory, &new_path)?;
                    }
                    writeln!(std, "Rename {:?} to {:?}", directory.directory, new_path)?;
                }
            }
        }
        Commands::List => directory
            .get_files()
            .get_sorted::<ByCreatedDate<&File>>()
            .into_iter()
            .try_for_each(|File { path, created }| writeln!(std, "{path:?}: Created {created}"))?,
        Commands::Interval => match directory.get_files().interval() {
            Some(interval) => writeln!(
                std,
                "from: {}, to: {} ({} days)",
                interval.from,
                interval.to,
                interval.delta().num_days()
            )?,
            None => writeln!(err, "Not enaught files to check")?,
        },
        Commands::Check {
            max_interval: max_days,
        } => match directory
            .get_files()
            .interval()
            .map(|interval| interval.delta())
        {
            Some(delta) if delta.abs().num_days() <= max_days.into() => {
                writeln!(std, "OK")?;
            }
            Some(delta) => {
                writeln!(err, "Delta is: {} days", delta.num_days())?;
            }
            None => {
                writeln!(err, "There is no files to check for interval")?;
            }
        },
        Commands::FilesRename { dry_run, name } => {
            directory
                .get_files()
                .rename_files(name.as_ref().map_or(directory.name()?, |n| n.as_str()))?
                .into_iter()
                .try_for_each(|RenamedFile(file, new_path)| {
                    if !dry_run {
                        fs::rename(&file.path, &new_path)?;
                    }
                    writeln!(std, "Rename file {:?} => {:?}", file.path, new_path)?;
                    Ok::<(), anyhow::Error>(())
                })?;
            let _files = directory.get_files();
        }
        Commands::MoveByDays { dry_run } => directory
            .get_files()
            .move_by_days()
            .into_iter()
            .flatten()
            .try_for_each(|RenamedFile(file, new_path)| {
                if let Some(parent) = &new_path.parent() {
                    if !dry_run {
                        fs::create_dir_all(parent)?;
                        fs::rename(&file.path, &new_path)?;
                    }
                    writeln!(std, "Move file {:?} => {:?}", file.path, new_path)?;
                }
                Ok::<(), anyhow::Error>(())
            })?,
    }
    Ok(())
}
