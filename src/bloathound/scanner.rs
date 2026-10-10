#![allow(unused)]
use core::time;
use indicatif::{ProgressBar, ProgressStyle};
use jwalk::WalkDir;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, HashSet},
    fs::File,
    path::{Path, PathBuf},
    time::SystemTime,
};
use walkdir::DirEntry;

#[derive(Clone, Eq, PartialEq)]
pub struct FileEntry {
    pub size: u64,
    pub path: PathBuf,
}

impl Ord for FileEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.size.cmp(&other.size)
    }
}

impl PartialOrd for FileEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Scanner {
    path: PathBuf,
    ignore_extensions: Option<HashSet<String>>,
    top_files: BinaryHeap<Reverse<FileEntry>>,
    limit: usize,
}

impl Scanner {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            ignore_extensions: None,
            top_files: BinaryHeap::new(),
            limit: 100,
        }
    }

    pub fn ignore_extension(&mut self, ext: impl AsRef<str>) {
        let normalized = ext.as_ref().trim_start_matches('.').to_lowercase();
        self.ignore_extensions
            .get_or_insert_with(HashSet::new)
            .insert(normalized);
    }

    // limits the amount of files to store in top files
    // default is 100
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    pub fn scan(&mut self) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴f⠦⠧⠇")
                .template("{spinner:.green} [{elapsed_precise}] {pos} files scanned ({prefix})")
                .unwrap(),
        );

        let mut total_bytes: u64 = 0;
        let mut file_count: u64 = 0;
        let now = SystemTime::now();

        let walker = jwalk::WalkDir::new(&self.path)
            .process_read_dir(|_depth, _path, _state, children| {
                children.retain(|dir_entry_result| {
                    if let Ok(entry) = dir_entry_result {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if Self::is_system_dir(file_name) {
                                return false;
                            }
                        }
                    }
                    true
                });
            })
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            if entry.file_type().is_file() {
                if self.should_ignore(&entry.path()) {
                    continue;
                }

                file_count += 1;

                if let Ok(metadata) = entry.metadata() {
                    let size = metadata.len();
                    total_bytes += size;
                    let path = entry.path().to_path_buf();

                    let file_entry = FileEntry { size, path };

                    if self.top_files.len() < self.limit {
                        self.top_files.push(Reverse(file_entry));
                    } else if let Some(Reverse(smallest)) = self.top_files.peek() {
                        if size > smallest.size {
                            self.top_files.pop();
                            self.top_files.push(Reverse(file_entry));
                        }
                    }
                }

                if file_count % 2000 == 0 {
                    pb.set_position(file_count);
                    pb.set_prefix(Self::format_bytes(total_bytes));
                }
            }
        }

        pb.finish_and_clear();

        println!(
            "Scan Complete! Scanned {} files. Total size: {}. Took: {} sec.",
            file_count,
            Self::format_bytes(total_bytes),
            now.elapsed().unwrap().as_secs()
        );
    }

    pub fn print_top_files(&self) {
        let mut sorted_files: Vec<FileEntry> =
            self.top_files.iter().map(|item| item.0.clone()).collect();
        sorted_files.sort_by(|a, b| b.size.cmp(&a.size));

        println!("");
        for (i, file) in sorted_files.iter().enumerate() {
            println!(
                "{}. [{}] {}",
                i + 1,
                Self::format_bytes(file.size),
                file.path.display()
            );
        }
    }

    pub fn get_top_files(&self) -> Vec<FileEntry> {
        let mut sorted_files: Vec<FileEntry> =
            self.top_files.iter().map(|item| item.0.clone()).collect();
        sorted_files.sort_by(|a, b| b.size.cmp(&a.size));

        return sorted_files.to_vec();
    }

    pub fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;
        const TB: u64 = GB * 1024;

        if bytes >= TB {
            format!("{:.2} TB", bytes as f64 / TB as f64)
        } else if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    fn should_ignore(&self, path: &Path) -> bool {
        let ignore_set = match &self.ignore_extensions {
            Some(set) if !set.is_empty() => set,
            _ => return false,
        };

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            return ignore_set.contains(&ext_lower);
        }

        false
    }

    fn is_system_dir(file_name: &str) -> bool {
        #[cfg(target_os = "windows")]
        const WINDOWS_RESTRICTED: &[&str] = &[
            "windows",
            "program files",
            "program files (x86)",
            "$recycle.bin",
            "system volume information",
        ];

        if WINDOWS_RESTRICTED
            .iter()
            .any(|&folder| file_name.eq_ignore_ascii_case(folder))
        {
            return false;
        }

        #[cfg(target_os = "linux")]
        {
            const LINUX_RESTRICTED: &[&str] = &["proc", "sys", "dev"];
            if LINUX_RESTRICTED
                .iter()
                .any(|&folder| file_name.eq_ignore_ascii_case(folder))
            {
                return false;
            }
        }

        false
    }
}
