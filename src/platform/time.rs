//! UTC 시각 표기 (외부 crate 없이 Howard Hinnant civil_from_days).

/// (년, 월, 일, 시, 분, 초)
pub fn utc_parts(secs: u64) -> (i64, i64, i64, u64, u64, u64) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, m, s)
}

/// Unix 시각 → `YYYY-MM-DD HH:MM:SS UTC`
pub fn format_utc(secs: u64) -> String {
    let (y, mo, d, h, m, s) = utc_parts(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02} UTC")
}

/// 분 단위 (폴더 목록용)
pub fn format_utc_minutes(secs: u64) -> String {
    let (y, mo, d, h, m, _) = utc_parts(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_formatting() {
        assert_eq!(format_utc_minutes(1_792_253_674), "2026-10-17 16:14");
        assert_eq!(format_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(format_utc(1_792_253_674), "2026-10-17 16:14:34 UTC");
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00 UTC");
    }
}
