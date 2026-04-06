use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::thread;
use std::time::Duration;

use console::style;

use doh_rs::runtime;

pub fn run(follow: bool, lines: usize) -> anyhow::Result<()> {
    let log_path = runtime::log_path();

    if !log_path.exists() {
        println!(
            "{} No log file found at {}",
            style("!").yellow(),
            log_path.display()
        );
        println!("  The proxy has not been started yet, or logging hasn't written yet.");
        return Ok(());
    }

    let file = File::open(&log_path)?;
    let mut reader = BufReader::new(file);

    // Show the last N lines
    let last_n_lines = collect_last_lines(&mut reader, lines)?;
    for line in &last_n_lines {
        print_log_line(line);
    }

    if !follow {
        return Ok(());
    }

    // Seek to end and follow new content
    reader.seek(SeekFrom::End(0))?;

    println!(
        "\n{} Following {} — Ctrl+C to quit",
        style("↓").cyan(),
        log_path.display()
    );

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => {
                // No new content — wait and retry
                thread::sleep(Duration::from_millis(100));
            }
            Ok(_) => {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    print_log_line(trimmed);
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
}

/// Read the entire file line-by-line and return the last `n` lines.
fn collect_last_lines(reader: &mut BufReader<File>, n: usize) -> anyhow::Result<Vec<String>> {
    reader.seek(SeekFrom::Start(0))?;
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = lines.len().saturating_sub(n);
    Ok(lines[start..].to_vec())
}

/// Apply color based on log level prefix found in the line.
fn print_log_line(line: &str) {
    if line.contains(" ERROR ") || line.contains(" error ") {
        println!("{}", style(line).red());
    } else if line.contains(" WARN ") || line.contains(" warn ") {
        println!("{}", style(line).yellow());
    } else if line.contains(" DEBUG ") || line.contains(" debug ") {
        println!("{}", style(line).dim());
    } else {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_log_file(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for l in lines {
            writeln!(f, "{l}").unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn collect_last_lines_returns_at_most_n() {
        let content: Vec<&str> = (0..30).map(|_| "line content").collect();
        let log = make_log_file(&content);
        let file = File::open(log.path()).unwrap();
        let mut reader = BufReader::new(file);
        let result = collect_last_lines(&mut reader, 10).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn collect_last_lines_when_fewer_than_n() {
        let log = make_log_file(&["a", "b", "c"]);
        let file = File::open(log.path()).unwrap();
        let mut reader = BufReader::new(file);
        let result = collect_last_lines(&mut reader, 20).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn collect_last_lines_empty_file() {
        let log = make_log_file(&[]);
        let file = File::open(log.path()).unwrap();
        let mut reader = BufReader::new(file);
        let result = collect_last_lines(&mut reader, 10).unwrap();
        assert_eq!(result.len(), 0);
    }
}
