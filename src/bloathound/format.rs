// I decided to move the formatting logic here
// there was an inconsistent split across 2 implementations
// and it was pointless, plus, this is far more elegant than
// if val > 1_000_000_000

const KB: u64 = 1024;
const MB: u64 = KB * 1024;
const GB: u64 = MB * 1024;
const TB: u64 = GB * 1024;

pub fn format_bytes(bytes: u64) -> String {
    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_largest_fitting_unit() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(MB), "1.00 MB");
        assert_eq!(format_bytes(GB), "1.00 GB");
        assert_eq!(format_bytes(TB), "1.00 TB");
    }

    #[test]
    fn unit_boundaries_do_not_round_past_their_tier() {
        assert_eq!(format_bytes(MB - 1), "1024.00 KB");
        assert_eq!(format_bytes(GB - 1), "1024.00 MB");
        assert_eq!(format_bytes(TB - 1), "1024.00 GB");
    }

    #[test]
    fn largest_unit_saturates_instead_of_rolling_over() {
        assert_eq!(format_bytes(1024 * TB), "1024.00 TB");
    }

    #[test]
    fn drive_capacities_match_the_windows_file_manager() {
        assert_eq!(format_bytes(1_000_000_000_000), "931.32 GB");
        assert_eq!(format_bytes(500_000_000_000), "465.66 GB");
        assert_eq!(format_bytes(500_000_000), "476.84 MB");
        assert_eq!(format_bytes(16_000_000), "15.26 MB");
    }
}
