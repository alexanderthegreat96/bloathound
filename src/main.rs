#![allow(unused)]
use clap::{Parser, Subcommand};
use dialoguer::{Confirm, MultiSelect, Select, console::Term};
use figlet_rs::Toilet;
use std::io::{self, Write};
use std::path::PathBuf;

use crate::bloathound::{
    disks::Drives,
    extensions::Extensions,
    scanner::{FileEntry, Scanner},
};

mod bloathound;

const APP_NAME: &str = "BLOATHOUND";
const APP_VERSION: &str = "v0.0.1";
const ITEM_PREFIX_WIDTH: usize = 8;

#[derive(Parser, Debug)]
#[command(
    name = "Cleaner",
    version = "1.0",
    about = "Fast disk analyzer and cleaner"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Interactive,
    Scan {
        #[arg(short, long)]
        path: PathBuf,
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
}

fn main() {
    let future_font = Toilet::future().unwrap();
    println!(
        "{}",
        future_font
            .convert(&format!("{APP_NAME} {APP_VERSION}"))
            .unwrap()
    );

    println!(
        "Disk scan & analysis tool. Will scan for the top largest files to enable you to delete them"
    );
    println!("");

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Interactive) {
        Commands::Interactive => {
            interactive_flow();
        }
        Commands::Scan { path, limit } => {
            scanner_workflow(path, limit);
        }
    }
}

fn interactive_flow() {
    let drives = bloathound::disks::Drives::new();
    let disks = drives.get_drives();

    if disks.is_empty() {
        println!("Unable to get disk data");
        return;
    }

    let drive_labels: Vec<String> = disks
        .iter()
        .map(|drive| {
            format!(
                "{} [{}] — Free: {:.2} {} / Total: {:.2} {}",
                drive.get_mount_point().display(),
                drive.get_name(),
                drive.get_free_space(),
                drive.get_free_space_kind(),
                drive.get_total_space(),
                drive.get_total_space_kind(),
            )
        })
        .collect();

    let drive_selector = Select::new()
        .with_prompt("Select a drive to scan")
        .items(&drive_labels)
        .interact()
        .unwrap();

    let target_path = disks[drive_selector].get_mount_point().to_path_buf();
    println!("Selected target: {:?}", target_path);

    let mut scanner = Scanner::new(target_path);

    let available_extensions = Extensions::all();
    let extension_labels: Vec<String> = available_extensions
        .iter()
        .map(|ext| format!(".{}", ext.to_extension()))
        .collect();

    let selected_extensions = MultiSelect::new()
        .with_prompt("Select which file extensions to IGNORE during the scan")
        .items(&extension_labels)
        .max_length(10)
        .interact()
        .unwrap();

    for idx in selected_extensions {
        let ext_to_ignore = available_extensions[idx];
        scanner.ignore_extension(ext_to_ignore.to_extension());
    }

    let available_limits = vec![50, 100, 150, 200, 250, 300];
    let available_limits_labels: Vec<String> = available_limits
        .iter()
        .map(|limit| format!("{}", limit))
        .collect();

    let selected_limit = Select::new()
        .with_prompt("Chose a limit for the amount of largest files")
        .items(&available_limits_labels)
        .interact()
        .unwrap();

    let chosen_limit = available_limits[selected_limit];
    scanner.set_limit(chosen_limit);

    scanner.scan();

    io::stdout().flush().unwrap();
    println!();

    let top_files = scanner.get_top_files();

    if top_files.is_empty() {
        println!("No files found to display.");
        return;
    }

    // nasty bug fix
    // dialog redraws so we need to do this
    // to prevent printing Select files to delete n times
    // each time we go up and down
    let label_width = (Term::stdout().size().1 as usize)
        .saturating_sub(ITEM_PREFIX_WIDTH)
        .max(20);

    let file_labels: Vec<String> = top_files
        .iter()
        .map(|file| {
            elide_middle(
                &format!(
                    "[{}] {}",
                    Scanner::format_bytes(file.size),
                    file.path.display()
                ),
                label_width,
            )
        })
        .collect();

    let selected_files = MultiSelect::new()
        .with_prompt("Select files to delete and press ENTER")
        .items(&file_labels)
        .max_length(20)
        .interact()
        .unwrap();

    if selected_files.is_empty() {
        println!("No files selected.");
        return;
    }

    println!("\nFiles queued for deletion:");
    for &idx in &selected_files {
        let file = &top_files[idx];
        println!(
            "  - [{}] {}",
            Scanner::format_bytes(file.size),
            file.path.display()
        );
    }
    println!();

    let confirmed = Confirm::new()
        .with_prompt("Are you sure you want to delete the files?")
        .default(false)
        .interact()
        .unwrap();

    if confirmed {
        println!("\nDeleting files...");
        for &idx in &selected_files {
            let target_file = &top_files[idx];
            match std::fs::remove_file(&target_file.path) {
                Ok(_) => println!("  [DELETED] {:?}", target_file.path),
                Err(e) => eprintln!("  [ERROR] Failed to delete {:?}: {}", target_file.path, e),
            }
        }
        println!("\nCleanup complete!");
    } else {
        println!("\nOperation cancelled. No files were deleted.");
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

fn scanner_workflow(path: PathBuf, limit: usize) {
    println!("Not implemented yet")
}
