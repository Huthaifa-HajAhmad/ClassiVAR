//! A calendar-date check shared by the rule file and the gene sources.

/// Is `text` a real date written `YYYY-MM-DD`? The month and the day must exist
/// in that year, so `2024-02-30` and `2023-02-29` are refused and `2024-02-29` is not.
pub fn is_calendar_date(text: &str) -> bool {
    let layout_ok = text.len() == 10
        && text.char_indices().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
    if !layout_ok {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) =
        (text[..4].parse::<u32>(), text[5..7].parse::<u32>(), text[8..].parse::<u32>())
    else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[cfg(test)]
mod tests {
    use super::is_calendar_date;

    #[test]
    fn real_dates_pass() {
        assert!(is_calendar_date("2026-11-11"));
        assert!(is_calendar_date("2024-02-29"));
        assert!(is_calendar_date("2000-02-29"));
        assert!(is_calendar_date("2026-12-31"));
    }

    #[test]
    fn days_that_do_not_exist_are_refused() {
        assert!(!is_calendar_date("2024-02-30"));
        assert!(!is_calendar_date("2023-02-29"));
        assert!(!is_calendar_date("1900-02-29"));
        assert!(!is_calendar_date("2026-04-31"));
        assert!(!is_calendar_date("2026-13-01"));
        assert!(!is_calendar_date("2026-00-10"));
        assert!(!is_calendar_date("2026-01-00"));
    }

    #[test]
    fn text_that_is_not_laid_out_as_a_date_is_refused() {
        assert!(!is_calendar_date(""));
        assert!(!is_calendar_date("2026-1-11"));
        assert!(!is_calendar_date("2026/11/11"));
        assert!(!is_calendar_date("2026-11-1x"));
        assert!(!is_calendar_date("２０２６-11-11"));
    }
}
