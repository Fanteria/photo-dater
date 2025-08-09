use anyhow::Result;
use std::env;
use std::io;

/// Main entry point that runs the photo_dater application.
///
/// Passes command line arguments and standard I/O streams to the library's
/// run function for processing.
fn main() -> Result<()> {
    photo_dater::run(env::args(), io::stdout(), io::stderr())
}
