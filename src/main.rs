mod directory;
mod file;
mod files;
mod files_interval;

use anyhow::Result;
use clap::{Parser, Subcommand};
use file::File;
use std::{fs, ops::Deref, path::PathBuf, process::exit};

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

fn main() -> Result<()> {
    let Cli { cmd, directory } = Cli::parse();
    let mut directory = Directory::try_from(directory)?;
    match cmd {
        Commands::Status => match directory.name_status() {
            Ok(directory::NameStatus::Valid) => println!("Date is valid"),
            Ok(directory::NameStatus::Invalid) => println!("Date is set but is invalid"),
            Ok(directory::NameStatus::None) => println!("Date is not set"),
            Err(e) => println!("Failed to get status '{}'", e),
        },
        Commands::Rename {
            max_interval,
            dry_run,
        } => {
            let (status, new_path) = directory.rename(max_interval)?;
            use directory::NameStatus as NS;
            match status {
                NS::Valid => println!("Directory already have right date"),
                NS::Invalid => println!("Directory already have date, but it is not match content"),
                NS::None => {
                    if !dry_run {
                        fs::rename(&directory.directory, &new_path)?;
                    }
                    println!("Rename {:?} to {:?}", directory.directory, new_path);
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
                .for_each(|File { path, created }| println!("{path:?}: Created {created}"));
        }
        Commands::Interval => match directory.get_files().interval() {
            Some(interval) => println!(
                "from: {}, to: {} ({} days)",
                interval.from,
                interval.to,
                interval.delta().num_days()
            ),
            None => println!("Not enaught files to check"),
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
                    println!("OK");
                }
                Some(delta) => {
                    println!("Delta is: {} days", delta.num_days());
                    exit(1);
                }
                None => {
                    println!("There is no files to check for interval");
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
                        println!("Move file {:?} => {:?}", file.path, new_path);
                    }
                    Ok::<(), anyhow::Error>(())
                })?;
        }
    }
    Ok(())
}
