use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, max_term_width = 110)]
/// Tally counter
struct Args {
    /// Path to file where we will store the counter value (will be overwritten)
    #[arg()]
    path: PathBuf,

    #[arg()]
    start_value: Option<i64>,
}

#[derive(Debug)]
struct FileCounter {
    path: PathBuf,
    count: i64,
}

// A counter that persists the count in a text file
impl FileCounter {
    fn new(path: PathBuf, value: Option<i64>) -> Result<FileCounter, io::Error> {
        // Initial count precedence:
        //   1) `value` argument
        //   2) first line of file given by `path` argument
        //   3) 0

        let mut count: i64 = 0;

        if let Some(value) = value {
            count = value;
        } else if let Ok(file) = OpenOptions::new().read(true).open(&path) {
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                if let Ok(value) = line.trim_end().parse::<i64>() {
                    count = value;
                }
            }
        }

        let counter = Self { path, count };
        counter.persist()?;
        Ok(counter)
    }

    fn increment(&mut self) -> Result<(), io::Error> {
        self.count += 1;
        self.persist()?;
        Ok(())
    }

    fn persist(&self) -> Result<(), io::Error> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.path)?;

        writeln!(file, "{}", self.count)?;
        Ok(())
    }
}

fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let counter = FileCounter::new(args.path, args.start_value)?;

    dbg!(counter);

    Ok(())
}
