use anyhow::Result;
use std::env;
use std::io;

fn main() -> Result<()> {
    photo_dater::run(env::args(), io::stdout(), io::stderr())
}
