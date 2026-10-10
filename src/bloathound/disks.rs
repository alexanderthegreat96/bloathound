#![allow(unused)]
use std::path::{Path, PathBuf};
use sysinfo::Disks;

pub struct Drive {
    idx: usize,
    name: String,
    mount_point: PathBuf,
    file_system: String,
    total_space: f64,
    free_space: f64,
    used_space: f64,
    total_space_kind: &'static str,
    free_space_kind: &'static str,
    used_space_kind: &'static str,
}

impl Drive {
    pub fn get_idx(&self) -> usize {
        return self.idx;
    }

    pub fn get_name(&self) -> &str {
        return &self.name;
    }

    pub fn get_mount_point(&self) -> &Path {
        return &self.mount_point;
    }

    pub fn get_file_system(&self) -> &str {
        return &self.file_system;
    }

    pub fn get_total_space(&self) -> f64 {
        return self.total_space;
    }

    pub fn get_free_space(&self) -> f64 {
        return self.free_space;
    }

    pub fn get_used_space(&self) -> f64 {
        return self.used_space;
    }

    pub fn get_total_space_kind(&self) -> &str {
        return self.total_space_kind;
    }

    pub fn get_free_space_kind(&self) -> &str {
        return self.free_space_kind;
    }

    pub fn get_used_space_kind(&self) -> &str {
        return self.used_space_kind;
    }
}

pub struct Drives {
    all_disks: Vec<Drive>,
}

impl Drives {
    pub fn new() -> Self {
        let mut all_disks: Vec<Drive> = Vec::new();

        let disks = Disks::new_with_refreshed_list();
        let mut idx = 0;

        for disk in disks.list() {
            let name = disk.name().to_str().unwrap_or("Unknown").to_string();
            let mount_point = disk.mount_point().to_path_buf();
            let file_system = disk.file_system().to_str().unwrap_or("Unknown").to_string();

            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space - available_space;

            let (total_space, total_space_kind) = Self::format_bytes(total_space);
            let (free_space, free_space_kind) = Self::format_bytes(available_space);
            let (used_space, used_space_kind) = Self::format_bytes(used_space);

            let drive: Drive = Drive {
                idx,
                name,
                mount_point,
                file_system,
                total_space,
                free_space,
                used_space,
                total_space_kind,
                free_space_kind,
                used_space_kind,
            };

            all_disks.push(drive);
            idx += 1;
        }

        Self { all_disks }
    }

    pub fn get_drives(&self) -> &Vec<Drive> {
        return &self.all_disks;
    }

    pub fn print_drives(&self) {
        let drives = self.get_drives();

        if drives.is_empty() {
            println!("Unable to return any drives");
            return;
        }

        for drive in drives {
            println!(
                "[{}] {:<12} | Path: {:<18} | FS: {:<6} | Used: {:>6.2} {} | Free: {:>6.2} {} / Total: {:>6.2} {}",
                drive.get_idx(),
                drive.get_name(),
                drive.get_mount_point().display(),
                drive.get_file_system(),
                drive.get_used_space(),
                drive.get_used_space_kind(),
                drive.get_free_space(),
                drive.get_free_space_kind(),
                drive.get_total_space(),
                drive.get_total_space_kind(),
            );
        }
    }

    fn format_bytes(bytes: u64) -> (f64, &'static str) {
        let mut space: f64 = bytes as f64;
        let kind = if space >= 1_000_000_000_000_000.0 {
            space /= 1_000_000_000_000_000.0;
            "PB"
        } else if space >= 1_000_000_000_000.0 {
            space /= 1_000_000_000_000.0;
            "TB"
        } else {
            space /= 1_000_000_000.0;
            "GB"
        };

        (space, kind)
    }
}
