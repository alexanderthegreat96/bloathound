use std::path::Path;
use std::process::ExitCode;

use crate::bloathound::{
    format::format_bytes,
    scanner::{FileEntry, Scanner},
};

// everything that we're taking from the scanner
// propagating it through the cli and calling
// the actual methods, then returning it back
// to the CLI segment
pub fn run(path: &Path, limit: usize, ignore: &[String]) -> ExitCode {
    if let Err(message) = validate_scan_path(path) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
    }

    if limit == 0 {
        eprintln!("error: --limit must be at least 1");
        return ExitCode::FAILURE;
    }

    let mut scanner = Scanner::new(path.to_path_buf());
    scanner.set_limit(limit);

    for extension in ignore {
        scanner.ignore_extension(extension);
    }

    println!("Scanning {}", path.display());
    if !ignore.is_empty() {
        println!("Ignoring: {}", ignore.join(", "));
    }

    scanner.scan();

    let top_files = scanner.get_top_files();

    if top_files.is_empty() {
        println!("\nNo files found under {}.", path.display());
        return ExitCode::SUCCESS;
    }

    print!("\n{}", render_top_files(&top_files));

    ExitCode::SUCCESS
}

fn validate_scan_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!(
            "path is not a directory: {} (pass the folder that contains it)",
            path.display()
        ));
    }

    Ok(())
}

fn render_top_files(files: &[FileEntry]) -> String {
    let sizes: Vec<String> = files.iter().map(|file| format_bytes(file.size)).collect();

    let index_width = files.len().to_string().len();
    let size_width = sizes.iter().map(|size| size.len()).max().unwrap_or(0);

    let mut out = format!("Top {} largest files:\n", files.len());

    for (position, (file, size)) in files.iter().zip(&sizes).enumerate() {
        out.push_str(&format!(
            "{:>index_width$}. [{:>size_width$}] {}\n",
            position + 1,
            size,
            file.path.display(),
        ));
    }

    let total: u64 = files.iter().map(|file| file.size).sum();
    out.push_str(&format!("\nListed files total: {}\n", format_bytes(total)));

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(size: u64, path: &str) -> FileEntry {
        FileEntry {
            size,
            path: PathBuf::from(path),
        }
    }

    #[test]
    fn validate_scan_path_accepts_a_directory() {
        assert!(validate_scan_path(Path::new(".")).is_ok());
    }

    #[test]
    fn validate_scan_path_rejects_a_missing_path() {
        let err = validate_scan_path(Path::new("./definitely-not-here-9f3a")).unwrap_err();
        assert!(err.contains("does not exist"), "{err}");
    }

    #[test]
    fn validate_scan_path_rejects_a_file() {
        let file = std::env::temp_dir().join("bloathound-validate-probe.bin");
        std::fs::write(&file, b"x").expect("write probe");

        let err = validate_scan_path(&file).unwrap_err();
        let _ = std::fs::remove_file(&file);

        assert!(err.contains("not a directory"), "{err}");
    }

    #[test]
    fn render_top_files_aligns_columns_and_totals() {
        let report = render_top_files(&[
            entry(2 * 1024 * 1024 * 1024, "C:\\big.bin"),
            entry(1024, "C:\\small.bin"),
        ]);

        let lines: Vec<&str> = report.lines().collect();
        assert_eq!(lines[0], "Top 2 largest files:");
        assert_eq!(lines[1], "1. [2.00 GB] C:\\big.bin");
        assert_eq!(lines[2], "2. [1.00 KB] C:\\small.bin");
        assert_eq!(lines[4], "Listed files total: 2.00 GB");
    }

    #[test]
    fn render_top_files_pads_indices_past_nine() {
        let files: Vec<FileEntry> = (0..10).map(|i| entry(1024, &format!("f{i}"))).collect();
        let report = render_top_files(&files);
        let lines: Vec<&str> = report.lines().collect();

        assert_eq!(lines[1], " 1. [1.00 KB] f0");
        assert_eq!(lines[10], "10. [1.00 KB] f9");
    }
}
