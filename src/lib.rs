mod directory;
mod file;
mod files;
mod files_interval;

use anyhow::Result;
use clap::{Parser, Subcommand};
use file::File;
use std::{ffi::OsString, fs, io, ops::Deref, path::PathBuf, process::exit};

use crate::{directory::Directory, file::ByCreatedDate};

#[derive(Subcommand, Clone, Debug)]
enum Commands {
    ///
    Status,
    ///
    Rename {
        /// Maximal interval in days
        #[arg(default_value = "0")]
        max_interval: u32,
        #[arg(short, long)]
        dry_run: bool,
    },
    ///
    List,
    ///
    Interval,
    ///
    Check {
        /// Maximal interval in days
        max_interval: u32,
    },
    ///
    FilesRename {
        #[arg(short, long)]
        dry_run: bool,
    },
    ///
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
    let mut directory = Directory::try_from(directory)?;
    match cmd {
        Commands::Status => match directory.name_status() {
            Ok(directory::NameStatus::Valid) => writeln!(std, "Date is valid")?,
            Ok(directory::NameStatus::Invalid) => writeln!(std, "Date is set but is invalid")?,
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
                NS::None => {
                    if !dry_run {
                        fs::rename(&directory.directory, &new_path)?;
                    }
                    writeln!(std, "Rename {:?} to {:?}", directory.directory, new_path)?;
                }
            }
        }
        Commands::List => {
            let files = directory.get_mut_files();
            let mut files: Vec<_> = files.iter().map(ByCreatedDate::<&File>).collect();
            files.sort();

            files
                .iter()
                .map(|a| a.deref())
                .try_for_each(|File { path, created }| {
                    writeln!(std, "{path:?}: Created {created}")
                })?;
        }
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
        } => {
            match directory
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
            }
        }
        Commands::FilesRename { dry_run: _ } => {
            // directory.name()
            let _files = directory.get_files();
        }
        Commands::MoveByDays { dry_run } => {
            directory
                .get_files()
                .move_by_days()
                .into_iter()
                .flatten()
                .try_for_each(|(file, new_path)| {
                    if let Some(parent) = &new_path.parent() {
                        if !dry_run {
                            fs::create_dir_all(parent)?;
                            fs::rename(&file.path, &new_path)?;
                        }
                        writeln!(std, "Move file {:?} => {:?}", file.path, new_path)?;
                    }
                    Ok::<(), anyhow::Error>(())
                })?;
        }
    }
    Ok(())
}
