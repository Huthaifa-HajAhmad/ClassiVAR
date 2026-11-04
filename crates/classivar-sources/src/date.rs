//! A tiny UTC date formatter, so we do not need a date library for one line of text.

use std::time::{SystemTime, UNIX_EPOCH};

/// `YYYY-MM-DD` in UTC. Dates before 1970 are not supported and give 1970-01-01.
pub fn utc_date(time: SystemTime) -> String {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Days since 1970-01-01 to (year, month, day). Algorithm by Howard Hinnant.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(seconds: u64) -> String {
        utc_date(UNIX_EPOCH + Duration::from_secs(seconds))
    }

    #[test]
    fn known_dates() {
        assert_eq!(at(0), "1970-01-01");
        assert_eq!(at(951_782_400), "2000-02-29"); // a leap day
        assert_eq!(at(1_762_128_000), "2025-11-03");
        assert_eq!(at(1_793_664_000), "2026-11-03");
        assert_eq!(at(1_793_663_999), "2026-11-02"); // the last second before
    }
}
