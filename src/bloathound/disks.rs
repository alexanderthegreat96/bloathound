use std::path::{Path, PathBuf};
use sysinfo::Disks;

use crate::bloathound::format::format_bytes;

#[allow(dead_code)]
pub struct Drive {
    idx: usize,
    name: String,
    mount_point: PathBuf,
    file_system: String,
    total_space: u64,
    free_space: u64,
    used_space: u64,
}

#[allow(dead_code)]
impl Drive {
    pub fn get_idx(&self) -> usize {
        self.idx
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_mount_point(&self) -> &Path {
        &self.mount_point
    }

    pub fn get_file_system(&self) -> &str {
        &self.file_system
    }

    pub fn get_total_space(&self) -> u64 {
        self.total_space
    }

    pub fn get_free_space(&self) -> u64 {
        self.free_space
    }

    pub fn get_used_space(&self) -> u64 {
        self.used_space
    }

    pub fn used_percent(&self) -> f64 {
        if self.total_space == 0 {
            return 0.0;
        }

        self.used_space as f64 / self.total_space as f64 * 100.0
    }

    pub fn summary(&self) -> String {
        let label = if self.name.is_empty() {
            String::new()
        } else {
            format!(" [{}]", self.name)
        };

        format!(
            "{}{} — Free: {} / Total: {} ({:.0}% used)",
            self.mount_point.display(),
            label,
            format_bytes(self.free_space),
            format_bytes(self.total_space),
            self.used_percent(),
        )
    }
}

pub struct Drives {
    all_disks: Vec<Drive>,
}

impl Drives {
    pub fn new() -> Self {
        let disks = Disks::new_with_refreshed_list();

        let all_disks = disks
            .list()
            .iter()
            .enumerate()
            .map(|(idx, disk)| {
                let total_space = disk.total_space();
                let free_space = disk.available_space();

                Drive {
                    idx,
                    name: disk.name().to_str().unwrap_or("Unknown").to_string(),
                    mount_point: disk.mount_point().to_path_buf(),
                    file_system: disk.file_system().to_str().unwrap_or("Unknown").to_string(),
                    total_space,
                    free_space,
                    used_space: total_space.saturating_sub(free_space),
                }
            })
            .collect();

        Self { all_disks }
    }

    pub fn get_drives(&self) -> &[Drive] {
        &self.all_disks
    }

    #[allow(dead_code)]
    pub fn print_drives(&self) {
        let drives = self.get_drives();

        if drives.is_empty() {
            println!("Unable to return any drives");
            return;
        }

        for drive in drives {
            println!(
                "[{}] {:<12} | Path: {:<18} | FS: {:<6} | Used: {:>10} | Free: {:>10} / Total: {:>10}",
                drive.get_idx(),
                drive.get_name(),
                drive.get_mount_point().display(),
                drive.get_file_system(),
                format_bytes(drive.get_used_space()),
                format_bytes(drive.get_free_space()),
                format_bytes(drive.get_total_space()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(total: u64, free: u64) -> Drive {
        Drive {
            idx: 0,
            name: "Test".to_string(),
            mount_point: PathBuf::from("T:\\"),
            file_system: "NTFS".to_string(),
            total_space: total,
            free_space: free,
            used_space: total.saturating_sub(free),
        }
    }

    #[test]
    fn used_percent_is_derived_from_raw_bytes() {
        assert_eq!(drive(1000, 250).used_percent(), 75.0);
        assert_eq!(drive(1000, 1000).used_percent(), 0.0);
        assert_eq!(drive(1000, 0).used_percent(), 100.0);
    }

    #[test]
    fn used_percent_handles_a_zero_sized_drive() {
        assert_eq!(drive(0, 0).used_percent(), 0.0);
    }

    #[test]
    fn used_space_never_underflows() {
        assert_eq!(drive(100, 500).get_used_space(), 0);
    }

    #[test]
    fn summary_renders_through_the_shared_formatter() {
        let summary = drive(1024 * 1024 * 1024, 512 * 1024 * 1024).summary();

        assert!(summary.contains("Free: 512.00 MB"), "{summary}");
        assert!(summary.contains("Total: 1.00 GB"), "{summary}");
        assert!(summary.contains("50% used"), "{summary}");
        assert!(summary.starts_with("T:\\ [Test] —"), "{summary}");
    }

    #[test]
    fn summary_omits_the_bracket_for_an_unnamed_volume() {
        let mut unnamed = drive(100, 50);
        unnamed.name = String::new();

        let summary = unnamed.summary();
        assert!(!summary.contains("[]"), "{summary}");
        assert!(summary.starts_with("T:\\ —"), "{summary}");
    }
}
