use dialoguer::{Confirm, MultiSelect, Select, console::Term};

use crate::bloathound::{
    cli::prompt,
    disks::Drives,
    extensions::Extensions,
    format::format_bytes,
    scanner::{FileEntry, Scanner},
};

const ITEM_PREFIX_WIDTH: usize = 8;

pub fn run(permanent: bool) {
    let drives = Drives::new();
    let disks = drives.get_drives();

    if disks.is_empty() {
        println!("Unable to get disk data");
        return;
    }

    let drive_labels: Vec<String> = disks.iter().map(|drive| drive.summary()).collect();

    let drive_selector = prompt(
        Select::new()
            .with_prompt("Select a drive to scan")
            .items(&drive_labels)
            .interact(),
    );

    let target_path = disks[drive_selector].get_mount_point().to_path_buf();
    println!("Selected target: {}", target_path.display());

    let mut scanner = Scanner::new(target_path);

    let available_extensions = Extensions::all();
    let extension_labels: Vec<String> = available_extensions
        .iter()
        .map(|ext| format!(".{}", ext.to_extension()))
        .collect();

    let selected_extensions = prompt(
        MultiSelect::new()
            .with_prompt("Select which file extensions to IGNORE during the scan")
            .items(&extension_labels)
            .max_length(10)
            .interact(),
    );

    for idx in selected_extensions {
        let ext_to_ignore = available_extensions[idx];
        scanner.ignore_extension(ext_to_ignore.to_extension());
    }

    let available_limits = [50, 100, 150, 200, 250, 300];
    let available_limits_labels: Vec<String> = available_limits
        .iter()
        .map(|limit| limit.to_string())
        .collect();

    let selected_limit = prompt(
        Select::new()
            .with_prompt("Chose a limit for the amount of largest files")
            .items(&available_limits_labels)
            .interact(),
    );

    scanner.set_limit(available_limits[selected_limit]);
    scanner.scan();
    println!();

    let top_files = scanner.get_top_files();

    if top_files.is_empty() {
        println!("No files found to display.");
        return;
    }

    let file_labels = build_file_labels(&top_files);

    let selected_files = prompt(
        MultiSelect::new()
            .with_prompt("Select files to delete and press ENTER")
            .items(&file_labels)
            .max_length(20)
            .interact(),
    );

    if selected_files.is_empty() {
        println!("No files selected.");
        return;
    }

    let queued: Vec<&FileEntry> = selected_files.iter().map(|&idx| &top_files[idx]).collect();

    confirm_and_delete(&queued, permanent);
}

fn build_file_labels(top_files: &[FileEntry]) -> Vec<String> {
    // nasty bug fix
    // dialog redraws so we need to do this
    // to prevent printing Select files to delete n times
    // each time we go up and down
    let label_width = (Term::stderr().size().1 as usize)
        .saturating_sub(ITEM_PREFIX_WIDTH)
        .max(20);

    top_files
        .iter()
        .map(|file| {
            elide_middle(
                &format!("[{}] {}", format_bytes(file.size), file.path.display()),
                label_width,
            )
        })
        .collect()
}

fn confirm_and_delete(queued: &[&FileEntry], permanent: bool) {
    let queued_bytes: u64 = queued.iter().map(|file| file.size).sum();

    println!("\nFiles queued for deletion:");
    for file in queued {
        println!("  - [{}] {}", format_bytes(file.size), file.path.display());
    }
    println!(
        "\n{} file(s), {} total.",
        queued.len(),
        format_bytes(queued_bytes)
    );

    let action = if permanent {
        "PERMANENTLY delete"
    } else {
        "move to the recycle bin"
    };

    let confirmed = prompt(
        Confirm::new()
            .with_prompt(format!("Are you sure you want to {action} these files?"))
            .default(false)
            .interact(),
    );

    if !confirmed {
        println!("\nOperation cancelled. No files were deleted.");
        return;
    }

    println!();
    let mut removed = 0usize;
    let mut reclaimed = 0u64;

    for file in queued {
        let outcome = if permanent {
            std::fs::remove_file(&file.path).map_err(|err| err.to_string())
        } else {
            trash::delete(&file.path).map_err(|err| err.to_string())
        };

        match outcome {
            Ok(()) => {
                removed += 1;
                reclaimed += file.size;
                println!("  [REMOVED] {}", file.path.display());
            }
            Err(err) => eprintln!("  [ERROR] {}: {}", file.path.display(), err),
        }
    }

    println!(
        "\nCleanup complete! {}/{} file(s) removed, {} reclaimed.",
        removed,
        queued.len(),
        format_bytes(reclaimed)
    );

    if !permanent && removed > 0 {
        println!("Removed files are in the recycle bin if you need them back.");
    }
}

fn elide_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();

    if chars.len() <= max {
        return text.to_string();
    }
    if max <= 3 {
        return chars[..max].iter().collect();
    }

    let keep = max - 3;
    let head = keep / 2;
    let tail = keep - head;

    let mut out: String = chars[..head].iter().collect();
    out.push_str("...");
    out.extend(&chars[chars.len() - tail..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn elide_middle_leaves_short_text_alone() {
        assert_eq!(elide_middle("", 10), "");
        assert_eq!(elide_middle("[1 KB] a.txt", 40), "[1 KB] a.txt");
        assert_eq!(elide_middle("abcde", 5), "abcde");
    }

    #[test]
    fn elide_middle_never_exceeds_the_budget() {
        let long = format!("[1.23 GB] C:\\Users\\{}\\file.bin", "a".repeat(200));

        for max in [0, 1, 2, 3, 4, 5, 20, 60, 119, 120] {
            let out = elide_middle(&long, max);
            assert_eq!(
                out.chars().count(),
                max,
                "width {max} produced {} chars",
                out.chars().count()
            );
        }
    }

    #[test]
    fn elide_middle_keeps_both_ends() {
        let out = elide_middle("[1.23 GB] C:\\very\\long\\path\\to\\archive.zip", 30);

        assert_eq!(out.chars().count(), 30);
        assert!(out.starts_with("[1.23 GB]"), "lost the size tag: {out}");
        assert!(out.ends_with("archive.zip"), "lost the file name: {out}");
        assert!(out.contains("..."));
    }

    #[test]
    fn elide_middle_handles_multibyte_characters() {
        let text = "[1 KB] C:\\Users\\Ünïcødé\\файл-日本語-αβγδε.bin";

        for max in 0..text.chars().count() + 5 {
            let out = elide_middle(text, max);
            assert_eq!(out.chars().count(), max.min(text.chars().count()));
        }
    }

    #[test]
    fn build_file_labels_fits_every_row_in_the_terminal() {
        let files: Vec<FileEntry> = (0..5)
            .map(|i| FileEntry {
                size: 1024 * (i + 1),
                path: PathBuf::from(format!("C:\\Users\\{}\\file{i}.bin", "x".repeat(300))),
            })
            .collect();

        let width = (Term::stderr().size().1 as usize)
            .saturating_sub(ITEM_PREFIX_WIDTH)
            .max(20);

        for label in build_file_labels(&files) {
            assert!(
                label.chars().count() <= width,
                "label is {} chars, budget {width}: {label}",
                label.chars().count()
            );
        }
    }
}
