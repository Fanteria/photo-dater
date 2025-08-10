mod directory;
mod file;
mod files;
mod files_interval;

use crate::{
    directory::Directory,
    file::{ByCreatedDate, ByPath},
    files::RenamedFile,
};
use anyhow::Result;
use clap::{builder::styling::AnsiColor, Parser, Subcommand, ValueEnum};
use file::File;
use std::{ffi::OsString, fs, io, path::PathBuf};

#[derive(ValueEnum, Debug, Clone)]
enum RenameFileSort {
    ByPath,
    ByCreatedDate,
}

/// Available commands
#[derive(Subcommand, Clone, Debug)]
enum Commands {
    /// Check the status of directory naming based on contained files' dates
    Status,

    /// Rename directory based on the date range of contained files
    Rename {
        /// Maximum allowed interval in days between oldest and newest files
        #[arg(default_value = "0")]
        max_interval: u32,
        /// Preview the rename operation without actually performing it
        #[arg(short = 'D', long)]
        dry_run: bool,
    },

    /// List all files in the directory sorted by creation date
    List,

    /// Display the date interval (range) of files in the directory
    Interval,

    /// Check if the file date interval is within acceptable limits
    Check {
        /// Maximum allowed interval in days
        max_interval: u32,
    },

    /// Rename individual files with sequential numbering
    FilesRename {
        /// Preview the rename operation without actually performing it
        #[arg(short = 'D', long)]
        dry_run: bool,
        /// Base name for renaming files (uses directory name if not provided)
        #[arg(short, long)]
        name: Option<String>,
        /// Sorting criterion for file renaming (by-path or by-created-date)
        #[arg(short, long, default_value = "by-path")]
        sort_by: RenameFileSort,
        /// Number of digits for zero-padding sequential numbers.
        /// If not specified automatically calculates based on the total number of files.
        #[arg(short, long)]
        digits: Option<usize>,
    },

    /// Move files into subdirectories organized by creation date
    MoveByDays {
        /// Preview the move operation without actually performing it
        #[arg(short = 'D', long)]
        dry_run: bool,
    },
}

/// Command-line interface structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, styles = help_colors())]
struct Cli {
    /// Target directory to process
    #[arg(default_value = ".")]
    directory: PathBuf,

    /// The command to execute
    #[command(subcommand)]
    cmd: Commands,
}

fn help_colors() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .header(AnsiColor::Green.on_default().bold())
        .invalid(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default().bold())
        .valid(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Cyan.on_default())
}

/// Main application entry point that processes command-line arguments and executes commands.
///
/// This function parses command-line arguments, loads the target directory, and executes
/// the requested operation. It supports various photo organization tasks including
/// directory renaming, file listing, date checking, and file reorganization.
///
/// # Arguments
///
/// * `args` - Iterator over command-line arguments
/// * `std` - Writer for standard output messages
/// * `err` - Writer for error and status messages
pub fn run<I, T, WStd, WErr>(args: I, mut std: WStd, mut err: WErr) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
    WStd: io::Write,
    WErr: io::Write,
{
    let Cli { cmd, directory } = Cli::parse_from(args);
    let directory = Directory::try_from(directory)?;
    match cmd {
        Commands::Status => match directory.name_status() {
            Ok(directory::NameStatus::Valid) => writeln!(std, "Date is valid")?,
            Ok(directory::NameStatus::Invalid) => writeln!(std, "Date is set but is invalid")?,
            Ok(directory::NameStatus::SuperSet) => writeln!(std, "Date is set but is superset")?,
            Ok(directory::NameStatus::None) => writeln!(std, "Date is not set")?,
            Err(e) => writeln!(std, "Failed to get status '{e}'")?,
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
        Commands::FilesRename {
            dry_run,
            name,
            sort_by,
            digits,
        } => {
            let files = directory.get_files();
            let name = name.as_ref().map_or(directory.name()?, |n| n.as_str());
            match sort_by {
                RenameFileSort::ByPath => files.rename_files::<ByPath<&File>>(name, digits),
                RenameFileSort::ByCreatedDate => {
                    files.rename_files::<ByCreatedDate<&File>>(name, digits)
                }
            }?
            .into_iter()
            .try_for_each(|RenamedFile(file, new_path)| {
                if !dry_run {
                    fs::rename(&file.path, &new_path)?;
                }
                writeln!(std, "Rename file {:?} => {:?}", file.path, new_path)?;
                Ok::<(), anyhow::Error>(())
            })?;
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
