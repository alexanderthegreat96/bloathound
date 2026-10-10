use indicatif::{ProgressBar, ProgressStyle};
use jwalk::WalkDir;

use crate::bloathound::format::format_bytes;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

// this is to esentially prevent the user from
// accidentaly scanning and wiping data across different platforms
// this list was AI compiled, I double checked it, but, even eyballing it
// it seems to be correct. as of today, i tested this on windows
// and it's fine
#[cfg(windows)]
const RESTRICTED_ROOT_DIRS: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "$recycle.bin",
    "system volume information",
];

#[cfg(target_os = "linux")]
const RESTRICTED_ROOT_DIRS: &[&str] = &["proc", "sys", "dev", "run", "boot", "snap", "lost+found"];

#[cfg(target_os = "macos")]
const RESTRICTED_ROOT_DIRS: &[&str] = &[
    "dev",
    "system",
    "volumes",
    "net",
    "home",
    "cores",
    ".spotlight-v100",
    ".fseventsd",
    ".documentrevisions-v100",
    ".trashes",
    ".vol",
];

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
const RESTRICTED_ROOT_DIRS: &[&str] = &["proc", "sys", "dev"];

#[cfg(not(any(windows, unix)))]
const RESTRICTED_ROOT_DIRS: &[&str] = &[];

#[cfg(windows)]
const RESTRICTED_ROOT_FILES: &[&str] = &[
    "pagefile.sys",
    "hiberfil.sys",
    "swapfile.sys",
    "dumpstack.log",
    "dumpstack.log.tmp",
];

#[cfg(target_os = "linux")]
const RESTRICTED_ROOT_FILES: &[&str] = &["swapfile", "swap.img"];

#[cfg(not(any(windows, target_os = "linux")))]
const RESTRICTED_ROOT_FILES: &[&str] = &[];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileEntry {
    pub size: u64,
    pub path: PathBuf,
}

pub struct Scanner {
    path: PathBuf,
    ignore_extensions: Option<HashSet<String>>,
    top_files: BinaryHeap<Reverse<FileEntry>>, // much faster than a standard Hashset for this operation
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

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    pub fn scan(&mut self) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.green} [{elapsed_precise}] {pos} files scanned ({prefix})")
                .unwrap(),
        );

        // independent animation
        pb.enable_steady_tick(Duration::from_millis(100));

        let mut total_bytes: u64 = 0;
        let mut file_count: u64 = 0;
        let started = Instant::now();

        let scan_root = self.path.clone();
        let walker = WalkDir::new(&self.path)
            .process_read_dir(move |_depth, parent, _state, children| {
                if parent != scan_root.as_path() {
                    return;
                }

                children.retain(|child| match child {
                    // making use of &str here since we are borrowing a reference and it
                    // will prevent a string allocation which consumes a lot of resources
                    // in a scenario like this
                    Ok(entry) => match entry.file_name().to_str() {
                        Some(name) if entry.file_type().is_dir() => {
                            !Self::is_restricted_root_dir(name)
                        }
                        Some(name) => !Self::is_restricted_root_file(name),
                        None => true,
                    },
                    Err(_) => true,
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

                    self.consider(FileEntry { size, path });
                }

                // for performance reasons, i am printing only every 2k files
                // doing allocations and printing to stdout in this context
                // is tanking performance, but doing it this way, does give it a speed boost
                // EVEN THOUGH, it is not perfect
                if file_count.is_multiple_of(2000) {
                    pb.set_position(file_count);
                    pb.set_prefix(format_bytes(total_bytes));
                }
            }
        }

        pb.finish_and_clear();

        println!(
            "Scan Complete! Scanned {} files. Total size: {}. Took: {:.1} sec.",
            file_count,
            format_bytes(total_bytes),
            started.elapsed().as_secs_f64()
        );
    }

    fn consider(&mut self, entry: FileEntry) {
        if self.top_files.len() < self.limit {
            self.top_files.push(Reverse(entry));
        } else if let Some(Reverse(smallest)) = self.top_files.peek()
            && entry.size > smallest.size
        {
            self.top_files.pop();
            self.top_files.push(Reverse(entry));
        }
    }

    pub fn get_top_files(&self) -> Vec<FileEntry> {
        let mut sorted_files: Vec<FileEntry> =
            self.top_files.iter().map(|item| item.0.clone()).collect();
        sorted_files.sort_by_key(|file| Reverse(file.size));

        sorted_files
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

    fn is_restricted_root_dir(file_name: &str) -> bool {
        RESTRICTED_ROOT_DIRS
            .iter()
            .any(|&folder| file_name.eq_ignore_ascii_case(folder))
    }

    fn is_restricted_root_file(file_name: &str) -> bool {
        RESTRICTED_ROOT_FILES
            .iter()
            .any(|&name| file_name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new(tag: &str) -> Self {
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let unique = COUNTER.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "bloathound-test-{tag}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }

        fn file(&self, rel: &str, size: usize) -> PathBuf {
            let path = self.root.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create parent");
            }
            fs::write(&path, vec![b'x'; size]).expect("write file");
            path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn restricted_root_dirs_are_matched_case_insensitively() {
        #[cfg(windows)]
        {
            assert!(Scanner::is_restricted_root_dir("Windows"));
            assert!(Scanner::is_restricted_root_dir("windows"));
            assert!(Scanner::is_restricted_root_dir("WINDOWS"));
            assert!(Scanner::is_restricted_root_dir("Program Files (x86)"));
            assert!(Scanner::is_restricted_root_dir("$Recycle.Bin"));
            assert!(Scanner::is_restricted_root_dir("System Volume Information"));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(Scanner::is_restricted_root_dir("proc"));
            assert!(Scanner::is_restricted_root_dir("sys"));
            assert!(Scanner::is_restricted_root_dir("dev"));
            assert!(Scanner::is_restricted_root_dir("run"));
            assert!(Scanner::is_restricted_root_dir("snap"));
            assert!(Scanner::is_restricted_root_dir("lost+found"));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(Scanner::is_restricted_root_dir("System"));
            assert!(Scanner::is_restricted_root_dir("Volumes"));
            assert!(Scanner::is_restricted_root_dir("net"));
            assert!(Scanner::is_restricted_root_dir("home"));
            assert!(Scanner::is_restricted_root_dir(".Spotlight-V100"));
            assert!(Scanner::is_restricted_root_dir(".DocumentRevisions-V100"));
            assert!(Scanner::is_restricted_root_dir(".Trashes"));
            assert!(!Scanner::is_restricted_root_dir("proc"));
            assert!(!Scanner::is_restricted_root_dir("sys"));
        }

        assert!(!Scanner::is_restricted_root_dir("Users"));
        assert!(!Scanner::is_restricted_root_dir("my windows screenshots"));
        assert!(!Scanner::is_restricted_root_dir(""));
    }

    #[test]
    fn restricted_root_files_are_matched_case_insensitively() {
        #[cfg(windows)]
        {
            assert!(Scanner::is_restricted_root_file("pagefile.sys"));
            assert!(Scanner::is_restricted_root_file("PageFile.Sys"));
            assert!(Scanner::is_restricted_root_file("hiberfil.sys"));
            assert!(Scanner::is_restricted_root_file("swapfile.sys"));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(Scanner::is_restricted_root_file("swap.img"));
            assert!(Scanner::is_restricted_root_file("swapfile"));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(RESTRICTED_ROOT_FILES.is_empty());
            assert!(!Scanner::is_restricted_root_file("sleepimage"));
        }

        assert!(!Scanner::is_restricted_root_file("mydriver.sys"));
        assert!(!Scanner::is_restricted_root_file("pagefile.sys.bak"));
        assert!(!Scanner::is_restricted_root_file("notes.txt"));
    }

    #[test]
    fn every_platform_arm_defines_both_lists() {
        let _: &[&str] = RESTRICTED_ROOT_DIRS;
        let _: &[&str] = RESTRICTED_ROOT_FILES;

        for entry in RESTRICTED_ROOT_DIRS.iter().chain(RESTRICTED_ROOT_FILES) {
            assert_eq!(*entry, entry.to_lowercase(), "{entry} is not lowercase");
            assert!(!entry.is_empty());
        }
    }

    #[test]
    fn scan_hides_locked_system_files_at_the_root() {
        if RESTRICTED_ROOT_FILES.is_empty() {
            return;
        }

        let restricted = RESTRICTED_ROOT_FILES[0];
        let tree = TempTree::new("sysfile");
        tree.file("keep.bin", 1_000);
        tree.file(restricted, 50_000);
        tree.file(&format!("backups/{restricted}"), 7_000);

        let mut scanner = Scanner::new(tree.root.clone());
        scanner.scan();

        let sizes: Vec<u64> = scanner.get_top_files().iter().map(|f| f.size).collect();
        assert_eq!(sizes, vec![7_000, 1_000], "root system file was not hidden");
    }

    #[test]
    fn should_ignore_respects_the_extension_filter() {
        let mut scanner = Scanner::new(PathBuf::from("."));

        assert!(!scanner.should_ignore(Path::new("a.log")));

        scanner.ignore_extension(".LOG");

        assert!(scanner.should_ignore(Path::new("a.log")));
        assert!(scanner.should_ignore(Path::new("a.LOG")));
        assert!(scanner.should_ignore(Path::new("deep/dir/a.Log")));
        assert!(!scanner.should_ignore(Path::new("a.txt")));
        assert!(!scanner.should_ignore(Path::new("no_extension")));
        assert!(!scanner.should_ignore(Path::new(".log")));
    }

    #[test]
    fn file_entry_orders_by_size() {
        let small = FileEntry {
            size: 10,
            path: PathBuf::from("small"),
        };
        let large = FileEntry {
            size: 20,
            path: PathBuf::from("large"),
        };

        assert!(large > small);

        let same_size = FileEntry {
            size: 10,
            path: PathBuf::from("zzz"),
        };
        assert!(same_size > small);
        assert_eq!(small, small.clone());
    }

    #[test]
    fn consider_keeps_only_the_largest_files() {
        let mut scanner = Scanner::new(PathBuf::from("."));
        scanner.set_limit(3);

        for size in [50, 10, 40, 20, 30] {
            scanner.consider(FileEntry {
                size,
                path: PathBuf::from(format!("{size}")),
            });
        }

        let sizes: Vec<u64> = scanner.get_top_files().iter().map(|f| f.size).collect();
        assert_eq!(sizes, vec![50, 40, 30]);
    }

    #[test]
    fn scan_collects_largest_files_and_honours_the_limit() {
        let tree = TempTree::new("limit");
        tree.file("tiny.bin", 100);
        tree.file("medium.bin", 5_000);
        tree.file("nested/large.bin", 20_000);

        let mut scanner = Scanner::new(tree.root.clone());
        scanner.set_limit(2);
        scanner.scan();

        let top = scanner.get_top_files();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].size, 20_000);
        assert_eq!(top[1].size, 5_000);
    }

    #[test]
    fn scan_skips_ignored_extensions() {
        let tree = TempTree::new("ignore");
        tree.file("keep.bin", 1_000);
        tree.file("drop.log", 9_999);

        let mut scanner = Scanner::new(tree.root.clone());
        scanner.ignore_extension("log");
        scanner.scan();

        let top = scanner.get_top_files();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].size, 1_000);
    }

    #[test]
    fn scan_prunes_restricted_directories_at_the_root() {
        if RESTRICTED_ROOT_DIRS.is_empty() {
            return;
        }

        let restricted = RESTRICTED_ROOT_DIRS[0];
        let tree = TempTree::new("prune");
        tree.file("keep.bin", 1_000);
        tree.file(&format!("{restricted}/system.bin"), 50_000);
        tree.file(&format!("{restricted}/deeper/system.bin"), 60_000);

        let mut scanner = Scanner::new(tree.root.clone());
        scanner.scan();

        let top = scanner.get_top_files();
        assert_eq!(top.len(), 1, "restricted directory was walked into");
        assert_eq!(top[0].size, 1_000);
    }

    #[test]
    fn scan_only_prunes_restricted_names_at_the_root() {
        if RESTRICTED_ROOT_DIRS.is_empty() {
            return;
        }

        let restricted = RESTRICTED_ROOT_DIRS[0];
        let tree = TempTree::new("nested-prune");
        tree.file(&format!("games/{restricted}/save.bin"), 7_000);

        let mut scanner = Scanner::new(tree.root.clone());
        scanner.scan();

        let top = scanner.get_top_files();
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].size, 7_000);
    }
}
