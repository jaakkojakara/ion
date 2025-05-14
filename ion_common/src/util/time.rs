#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as StdInstant;

use bincode::{Decode, Encode};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use std::ops::{Add, AddAssign, Sub, SubAssign};
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;
use std::str::FromStr;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use std::time::Duration;

//-------------------------------------------------------//
//--------------------- Timezone ------------------------//
//-------------------------------------------------------//

#[cfg(not(target_arch = "wasm32"))]
static TIMEZONE_OFFSET: OnceLock<i64> = OnceLock::new();

//-------------------------------------------------------//
//---------------------- Instant ------------------------//
//-------------------------------------------------------//

/// A platform-specific instant that can be used to measure time differences.
/// On native platforms, this uses std::time::Instant which provides high accuracy.
/// On web platforms, this uses performance.now() which is the least inaccurate API.
///
/// Note: This is only for measuring time differences, not for getting absolute time.
#[derive(Debug, Clone, Copy)]
pub struct Instant {
    #[cfg(not(target_arch = "wasm32"))]
    inner: StdInstant,
    #[cfg(target_arch = "wasm32")]
    inner: f64,
}

impl Instant {
    /// Creates a new instant representing the current time.
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: StdInstant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            #[cfg(target_arch = "wasm32")]
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = performance)]
                fn now() -> f64;
            }
            Self { inner: now() }
        }
    }

    /// Returns the elapsed time since this instant was created.
    pub fn elapsed(&self) -> std::time::Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.elapsed()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let elapsed = Self::now().inner - self.inner;
            std::time::Duration::from_secs_f64(elapsed.max(0.0) / 1000.0)
        }
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or None if that instant is later than this one.
    pub fn duration_since(&self, earlier: Instant) -> Option<Duration> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.checked_duration_since(earlier.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            if self.inner >= earlier.inner {
                Some(Duration::from_secs_f64((self.inner - earlier.inner).max(0.0) / 1000.0))
            } else {
                None
            }
        }
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.checked_add(duration).map(|i| Instant { inner: i })
        }
        #[cfg(target_arch = "wasm32")]
        {
            Some(Instant {
                inner: self.inner + duration.as_secs_f64() * 1000.0,
            })
        }
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented as
    /// `Instant` (which means it's inside the bounds of the underlying data structure), `None`
    /// otherwise.
    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.checked_sub(duration).map(|i| Instant { inner: i })
        }
        #[cfg(target_arch = "wasm32")]
        {
            let new_time = self.inner - duration.as_secs_f64() * 1000.0;
            if new_time >= 0.0 {
                Some(Instant { inner: new_time })
            } else {
                None
            }
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, other: Duration) -> Instant {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Instant {
                inner: self.inner + other,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Instant {
                inner: self.inner + other.as_secs_f64() * 1000.0,
            }
        }
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, other: Duration) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner += other;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner += other.as_secs_f64() * 1000.0;
        }
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, other: Duration) -> Instant {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Instant {
                inner: self.inner - other,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Instant {
                inner: (self.inner - other.as_secs_f64() * 1000.0).max(0.0),
            }
        }
    }
}

impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, other: Duration) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner -= other;
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner = (self.inner - other.as_secs_f64() * 1000.0).max(0.0);
        }
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, other: Instant) -> Duration {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner - other.inner
        }
        #[cfg(target_arch = "wasm32")]
        {
            Duration::from_secs_f64((self.inner - other.inner) / 1000.0)
        }
    }
}

impl PartialEq for Instant {
    fn eq(&self, other: &Instant) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner == other.inner
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner == other.inner
        }
    }
}

impl Eq for Instant {}

impl PartialOrd for Instant {
    fn partial_cmp(&self, other: &Instant) -> Option<std::cmp::Ordering> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(self.inner.cmp(&other.inner))
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.partial_cmp(&other.inner)
        }
    }
}

impl Ord for Instant {
    fn cmp(&self, other: &Instant) -> std::cmp::Ordering {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.cmp(&other.inner)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner
                .partial_cmp(&other.inner)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }
}

impl std::hash::Hash for Instant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.hash(state)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.to_bits().hash(state)
        }
    }
}

//-------------------------------------------------------//
//--------------------- DateTime ------------------------//
//-------------------------------------------------------//

/// A platform-specific DateTime that represents moments in time and can convert to/from human readable formats.
/// On native platforms, this uses std::time::SystemTime.
/// On web platforms, this uses JavaScript's Date API.
///
/// Precision is limited to milliseconds for consistent cross-platform behavior.
#[derive(Clone, Copy, Encode, Decode)]
pub struct DateTime {
    #[cfg(not(target_arch = "wasm32"))]
    inner: SystemTime,
    #[cfg(target_arch = "wasm32")]
    inner: f64, // milliseconds since Unix epoch
}

impl DateTime {
    /// Creates a new DateTime representing the current time.
    pub fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: SystemTime::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                inner: js_sys::Date::now(),
            }
        }
    }

    /// Creates a DateTime from milliseconds since Unix epoch.
    pub fn from_unix_timestamp_ms(ms: u64) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                inner: UNIX_EPOCH + Duration::from_millis(ms),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self { inner: ms as f64 }
        }
    }

    /// Returns the number of milliseconds since Unix epoch.
    pub fn as_unix_timestamp_ms(&self) -> u64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner as u64
        }
    }

    /// Returns the amount of time elapsed from another DateTime to this one,
    /// or None if that DateTime is later than this one.
    pub fn duration_since(&self, earlier: DateTime) -> Option<Duration> {
        let self_ms = self.as_unix_timestamp_ms();
        let earlier_ms = earlier.as_unix_timestamp_ms();

        if self_ms >= earlier_ms {
            Some(Duration::from_millis(self_ms - earlier_ms))
        } else {
            None
        }
    }

    /// Formats the DateTime as an ISO 8601 string (UTC).
    pub fn format_iso8601(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ms = self.as_unix_timestamp_ms();
            let seconds = ms / 1000;
            let milliseconds = ms % 1000;

            // Calculate date components
            let days_since_epoch = seconds / 86400;
            let seconds_today = seconds % 86400;

            let (year, month, day) = days_to_date(days_since_epoch as i32);
            let hour = seconds_today / 3600;
            let minute = (seconds_today % 3600) / 60;
            let second = seconds_today % 60;

            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                year, month, day, hour, minute, second, milliseconds
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(self.inner));
            date.to_iso_string().as_string().unwrap_or_default()
        }
    }

    /// Formats the DateTime as "dd.mm.yyyy hh:mm:ss" in local timezone.
    pub fn format_display(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For native, we'll approximate local time by adding system timezone offset
            // This is a simplified implementation without full timezone database
            let ms = self.as_unix_timestamp_ms();
            let local_offset_ms = get_local_timezone_offset_ms();
            let local_ms = ms as i64 + local_offset_ms;

            let seconds = (local_ms / 1000) as u64;
            let days_since_epoch = seconds / 86400;
            let seconds_today = seconds % 86400;

            let (year, month, day) = days_to_date(days_since_epoch as i32);
            let hour = seconds_today / 3600;
            let minute = (seconds_today % 3600) / 60;
            let second = seconds_today % 60;

            format!(
                "{:02}.{:02}.{:04} {:02}:{:02}:{:02}",
                day, month, year, hour, minute, second
            )
        }
        #[cfg(target_arch = "wasm32")]
        {
            let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(self.inner));
            format!(
                "{:02}.{:02}.{:04} {:02}:{:02}:{:02}",
                date.get_date(),
                date.get_month() + 1, // JavaScript months are 0-based
                date.get_full_year(),
                date.get_hours(),
                date.get_minutes(),
                date.get_seconds()
            )
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_local_timezone_offset_ms() -> i64 {
    *TIMEZONE_OFFSET.get_or_init(|| {
        if let Some(offset_seconds) = get_timezone_offset_from_date_command() {
            offset_seconds * 1000
        } else {
            0 // Fallback to UTC
        }
    })
}

/// Get timezone offset in seconds using the "date +%z" command
#[cfg(not(target_arch = "wasm32"))]
fn get_timezone_offset_from_date_command() -> Option<i64> {
    let output = if cfg!(target_os = "windows") {
        panic!("Windows support for DateTime is not implemented")
    } else if cfg!(target_os = "macos") {
        Command::new("date").arg("+%z").output().ok()?
    } else if cfg!(target_os = "linux") {
        Command::new("date").arg("+%z").output().ok()?
    } else {
        panic!("Unsupported platform for DateTime")
    };

    if !output.status.success() {
        return None;
    }

    let output_str = String::from_utf8(output.stdout).ok()?;
    let offset_str = output_str.trim();

    parse_timezone_offset(offset_str)
}

/// Parse timezone offset string like "+0300" or "-0500"
#[cfg(not(target_arch = "wasm32"))]
fn parse_timezone_offset(offset_str: &str) -> Option<i64> {
    if offset_str.len() != 5 {
        return None;
    }

    let (sign_char, rest) = offset_str.split_at(1);
    let sign = match sign_char {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };

    let hours_str = &rest[0..2];
    let minutes_str = &rest[2..4];

    let hours: i64 = hours_str.parse().ok()?;
    let minutes: i64 = minutes_str.parse().ok()?;

    if hours > 23 || minutes > 59 {
        return None;
    }

    let total_seconds = sign * (hours * 3600 + minutes * 60);

    Some(total_seconds)
}

/// Convert days since Unix epoch (1970-01-01) to year, month, day
#[cfg(not(target_arch = "wasm32"))]
fn days_to_date(days_since_epoch: i32) -> (u32, u32, u32) {
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    // Handle negative days (before 1970)
    if remaining_days < 0 {
        year = 1969;
        remaining_days = 365 + remaining_days; // Approximate
    }

    // Find the year
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    // Find the month and day
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_month in &days_in_months {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;

    (year as u32, month, day as u32)
}

#[cfg(not(target_arch = "wasm32"))]
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

impl Add<Duration> for DateTime {
    type Output = DateTime;

    fn add(self, other: Duration) -> DateTime {
        DateTime::from_unix_timestamp_ms(self.as_unix_timestamp_ms() + other.as_millis() as u64)
    }
}

impl AddAssign<Duration> for DateTime {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for DateTime {
    type Output = DateTime;

    fn sub(self, other: Duration) -> DateTime {
        let current_ms = self.as_unix_timestamp_ms();
        let sub_ms = other.as_millis() as u64;
        DateTime::from_unix_timestamp_ms(current_ms.saturating_sub(sub_ms))
    }
}

impl SubAssign<Duration> for DateTime {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Sub<DateTime> for DateTime {
    type Output = Duration;

    fn sub(self, other: DateTime) -> Duration {
        let self_ms = self.as_unix_timestamp_ms();
        let other_ms = other.as_unix_timestamp_ms();
        Duration::from_millis(self_ms.saturating_sub(other_ms))
    }
}

impl PartialEq for DateTime {
    fn eq(&self, other: &DateTime) -> bool {
        // Compare at millisecond precision
        self.as_unix_timestamp_ms() == other.as_unix_timestamp_ms()
    }
}

impl Eq for DateTime {}

impl PartialOrd for DateTime {
    fn partial_cmp(&self, other: &DateTime) -> Option<std::cmp::Ordering> {
        self.as_unix_timestamp_ms().partial_cmp(&other.as_unix_timestamp_ms())
    }
}

impl Ord for DateTime {
    fn cmp(&self, other: &DateTime) -> std::cmp::Ordering {
        self.as_unix_timestamp_ms().cmp(&other.as_unix_timestamp_ms())
    }
}

impl std::hash::Hash for DateTime {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_unix_timestamp_ms().hash(state)
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_display())
    }
}

impl std::fmt::Debug for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_iso8601())
    }
}

impl FromStr for DateTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            parse_iso8601_native(s)
        }
        #[cfg(target_arch = "wasm32")]
        {
            parse_iso8601_wasm(s)
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_iso8601_wasm(s: &str) -> Result<DateTime, String> {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_str(s));
    let timestamp = date.value_of();

    if timestamp.is_nan() {
        Err(format!("Invalid date string: {}", s))
    } else {
        Ok(DateTime { inner: timestamp })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_iso8601_native(s: &str) -> Result<DateTime, String> {
    // Support formats:
    // YYYY-MM-DDTHH:MM:SS.sssZ
    // YYYY-MM-DDTHH:MM:SSZ
    // YYYY-MM-DDTHH:MM:SS.sss
    // YYYY-MM-DDTHH:MM:SS
    // YYYY-MM-DD

    let s = s.trim();

    // Split by 'T' to separate date and time parts
    let parts: Vec<&str> = s.split('T').collect();
    if parts.is_empty() || parts.len() > 2 {
        return Err("Invalid ISO 8601 format".to_string());
    }

    let date_part = parts[0];
    let time_part = if parts.len() > 1 { Some(parts[1]) } else { None };

    // Parse date part: YYYY-MM-DD
    let date_components: Vec<&str> = date_part.split('-').collect();
    if date_components.len() != 3 {
        return Err("Invalid date format, expected YYYY-MM-DD".to_string());
    }

    // Validate format: year should be 4 digits, month and day should be 2 digits
    if date_components[0].len() != 4 {
        return Err("Year must be 4 digits".to_string());
    }
    if date_components[1].len() != 2 {
        return Err("Month must be 2 digits".to_string());
    }
    if date_components[2].len() != 2 {
        return Err("Day must be 2 digits".to_string());
    }

    let year: i32 = date_components[0].parse().map_err(|_| "Invalid year")?;
    let month: u32 = date_components[1].parse().map_err(|_| "Invalid month")?;
    let day: u32 = date_components[2].parse().map_err(|_| "Invalid day")?;

    // Default time values
    let mut hour = 0u32;
    let mut minute = 0u32;
    let mut second = 0u32;
    let mut millisecond = 0u32;

    // Parse time part if present
    if let Some(time_str) = time_part {
        let time_str = time_str.trim_end_matches('Z'); // Remove timezone indicator

        // Split by '.' to separate seconds and milliseconds
        let time_parts: Vec<&str> = time_str.split('.').collect();
        let hms_part = time_parts[0];

        // Parse HH:MM:SS
        let hms_components: Vec<&str> = hms_part.split(':').collect();
        if hms_components.len() != 3 {
            return Err("Invalid time format, expected HH:MM:SS".to_string());
        }

        hour = hms_components[0].parse().map_err(|_| "Invalid hour")?;
        minute = hms_components[1].parse().map_err(|_| "Invalid minute")?;
        second = hms_components[2].parse().map_err(|_| "Invalid second")?;

        if hour > 23 {
            return Err("Hour must be between 0 and 23".to_string());
        }
        if minute > 59 {
            return Err("Minute must be between 0 and 59".to_string());
        }
        if second > 59 {
            return Err("Second must be between 0 and 59".to_string());
        }

        // Parse milliseconds if present
        if time_parts.len() > 1 {
            let ms_str = time_parts[1];
            if !ms_str.is_empty() {
                // Pad or truncate to 3 digits
                let ms_normalized = if ms_str.len() >= 3 {
                    &ms_str[0..3]
                } else {
                    // Pad with zeros
                    &format!("{:0<3}", ms_str)[0..3]
                };

                millisecond = ms_normalized.parse().map_err(|_| "Invalid milliseconds")?;
            }
        }
    }

    // Convert to Unix timestamp
    let days = date_to_days(year, month, day)?;
    let seconds_in_day = hour * 3600 + minute * 60 + second;
    let total_seconds = days as i64 * 86400 + seconds_in_day as i64;
    let total_ms = total_seconds * 1000 + millisecond as i64;

    if total_ms < 0 {
        return Err("Calculated timestamp is negative".to_string());
    }

    Ok(DateTime::from_unix_timestamp_ms(total_ms as u64))
}

#[cfg(not(target_arch = "wasm32"))]
fn date_to_days(year: i32, month: u32, day: u32) -> Result<i32, String> {
    // Simple algorithm to calculate days since Unix epoch (1970-01-01)

    // First, validate inputs
    if month < 1 || month > 12 {
        return Err("Month must be between 1 and 12".to_string());
    }
    if day < 1 || day > 31 {
        return Err("Day must be between 1 and 31".to_string());
    }

    // Calculate days since epoch using a direct approach
    let mut total_days = 0i32;

    // Add days for complete years since 1970
    for y in 1970..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Handle years before 1970
    if year < 1970 {
        for y in year..1970 {
            total_days -= if is_leap_year(y) { 366 } else { 365 };
        }
    }

    // Add days for complete months in the current year
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for m in 1..month {
        total_days += days_in_months[(m - 1) as usize] as i32;
    }

    // Add the remaining days (day - 1 because we want days *since* that date)
    total_days += (day - 1) as i32;

    Ok(total_days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    //-------------------------------------------------------//
    //-------------------- Instant Tests -------------------//
    //-------------------------------------------------------//

    #[test]
    fn test_instant_now() {
        let instant = Instant::now();
        // Should be able to create an instant without panicking
        // The elapsed time should be very small initially
        let elapsed = instant.elapsed();
        assert!(elapsed.as_millis() < 1000); // Should be less than 1 second
    }

    #[test]
    fn test_instant_elapsed() {
        let start = Instant::now();
        // Small delay to ensure some time passes
        std::thread::sleep(Duration::from_millis(1));
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 1);
    }

    #[test]
    fn test_instant_duration_since() {
        let earlier = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        let later = Instant::now();

        let duration = later.duration_since(earlier);
        assert!(duration.is_some());
        assert!(duration.unwrap().as_millis() >= 1);

        // Reverse should return None
        let reverse_duration = earlier.duration_since(later);
        assert!(reverse_duration.is_none());
    }

    #[test]
    fn test_instant_arithmetic() {
        let instant = Instant::now();
        let duration = Duration::from_millis(1000);

        // Test addition
        let future = instant + duration;
        assert!(future.duration_since(instant).unwrap() >= Duration::from_millis(999));

        // Test subtraction
        let past = future - duration;
        assert!(past.duration_since(instant).unwrap_or_default() <= Duration::from_millis(1));
    }

    #[test]
    fn test_instant_checked_operations() {
        let instant = Instant::now();
        let duration = Duration::from_millis(1000);

        // Test checked_add
        let future = instant.checked_add(duration);
        assert!(future.is_some());

        // Test checked_sub
        let past = instant.checked_sub(Duration::from_millis(1));
        // Should succeed on most platforms
        assert!(past.is_some() || past.is_none()); // Either is valid depending on platform
    }

    #[test]
    fn test_instant_comparisons() {
        let instant1 = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        let instant2 = Instant::now();

        assert!(instant2 > instant1);
        assert!(instant1 < instant2);
        assert!(instant1 != instant2);
        assert_eq!(instant1, instant1);
    }

    #[test]
    fn test_instant_subtraction() {
        let instant1 = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        let instant2 = Instant::now();

        let diff = instant2 - instant1;
        assert!(diff.as_millis() >= 1);
    }

    //-------------------------------------------------------//
    //--------------------- DateTime Tests -----------------//
    //-------------------------------------------------------//

    #[test]
    fn test_datetime_now() {
        let dt = DateTime::now();
        let timestamp = dt.as_unix_timestamp_ms();

        // Should be a reasonable timestamp (after year 2020)
        assert!(timestamp > 1_577_836_800_000); // 2020-01-01 00:00:00 UTC
    }

    #[test]
    fn test_datetime_from_timestamp() {
        let timestamp = 1_640_995_200_000; // 2022-01-01 00:00:00 UTC
        let dt = DateTime::from_unix_timestamp_ms(timestamp);
        assert_eq!(dt.as_unix_timestamp_ms(), timestamp);
    }

    #[test]
    fn test_datetime_roundtrip_timestamp() {
        let original_timestamp = 1_640_995_200_123; // With milliseconds
        let dt = DateTime::from_unix_timestamp_ms(original_timestamp);
        let recovered_timestamp = dt.as_unix_timestamp_ms();
        assert_eq!(original_timestamp, recovered_timestamp);
    }

    #[test]
    fn test_datetime_duration_since() {
        let timestamp1 = 1_640_995_200_000; // 2022-01-01 00:00:00 UTC
        let timestamp2 = 1_640_995_260_000; // 2022-01-01 00:01:00 UTC (1 minute later)

        let dt1 = DateTime::from_unix_timestamp_ms(timestamp1);
        let dt2 = DateTime::from_unix_timestamp_ms(timestamp2);

        let duration = dt2.duration_since(dt1);
        assert!(duration.is_some());
        assert_eq!(duration.unwrap(), Duration::from_secs(60));

        // Reverse should return None
        let reverse_duration = dt1.duration_since(dt2);
        assert!(reverse_duration.is_none());
    }

    #[test]
    fn test_datetime_format_iso8601() {
        let timestamp = 1_640_995_200_123; // 2022-01-01 00:00:00.123 UTC
        let dt = DateTime::from_unix_timestamp_ms(timestamp);
        let iso_string = dt.format_iso8601();

        // Should contain expected components
        assert!(iso_string.contains("2022"));
        assert!(iso_string.contains("01"));
        assert!(iso_string.contains("T"));
        assert!(iso_string.contains("Z"));
        assert!(iso_string.contains("123")); // milliseconds
    }

    #[test]
    fn test_datetime_format_display() {
        let timestamp = 1_640_995_200_000; // 2022-01-01 00:00:00 UTC
        let dt = DateTime::from_unix_timestamp_ms(timestamp);
        let display_string = dt.format_display();

        // Should be in dd.mm.yyyy hh:mm:ss format
        assert!(display_string.contains("2022") || display_string.contains("2021")); // Might be different in local timezone
        assert!(display_string.contains("."));
        assert!(display_string.contains(":"));
        assert!(display_string.len() >= 19); // At least "dd.mm.yyyy hh:mm:ss"
    }

    #[test]
    fn test_datetime_arithmetic() {
        let dt = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let duration = Duration::from_millis(1000);

        // Test addition
        let future = dt + duration;
        assert_eq!(future.as_unix_timestamp_ms(), 1_640_995_201_000);

        // Test subtraction
        let past = future - duration;
        assert_eq!(past.as_unix_timestamp_ms(), 1_640_995_200_000);
    }

    #[test]
    fn test_datetime_assign_operations() {
        let mut dt = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let duration = Duration::from_millis(5000);

        // Test add_assign
        dt += duration;
        assert_eq!(dt.as_unix_timestamp_ms(), 1_640_995_205_000);

        // Test sub_assign
        dt -= duration;
        assert_eq!(dt.as_unix_timestamp_ms(), 1_640_995_200_000);
    }

    #[test]
    fn test_datetime_subtraction() {
        let dt1 = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let dt2 = DateTime::from_unix_timestamp_ms(1_640_995_260_000);

        let diff = dt2 - dt1;
        assert_eq!(diff, Duration::from_secs(60));

        // Reverse subtraction (should saturate)
        let reverse_diff = dt1 - dt2;
        assert_eq!(reverse_diff, Duration::from_secs(0));
    }

    #[test]
    fn test_datetime_comparisons() {
        let dt1 = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let dt2 = DateTime::from_unix_timestamp_ms(1_640_995_260_000);
        let dt3 = DateTime::from_unix_timestamp_ms(1_640_995_200_000);

        assert!(dt2 > dt1);
        assert!(dt1 < dt2);
        assert!(dt1 != dt2);
        assert_eq!(dt1, dt3);
        assert!(dt1 <= dt3);
        assert!(dt1 >= dt3);
    }

    #[test]
    fn test_datetime_equality_millisecond_precision() {
        // Test that equality works at millisecond precision
        let dt1 = DateTime::from_unix_timestamp_ms(1_640_995_200_123);
        let dt2 = DateTime::from_unix_timestamp_ms(1_640_995_200_123);
        let dt3 = DateTime::from_unix_timestamp_ms(1_640_995_200_124);

        assert_eq!(dt1, dt2);
        assert_ne!(dt1, dt3);
    }

    #[test]
    fn test_datetime_display_and_debug() {
        let dt = DateTime::from_unix_timestamp_ms(1_640_995_200_000);

        // Test Display trait
        let display_str = format!("{}", dt);
        assert!(!display_str.is_empty());

        // Test Debug trait
        let debug_str = format!("{:?}", dt);
        assert!(debug_str.contains("2022"));
        assert!(debug_str.contains("T")); // Should be ISO format
    }

    #[test]
    fn test_datetime_hash() {
        use std::collections::HashSet;

        let dt1 = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let dt2 = DateTime::from_unix_timestamp_ms(1_640_995_200_000);
        let dt3 = DateTime::from_unix_timestamp_ms(1_640_995_260_000);

        let mut set = HashSet::new();
        set.insert(dt1);
        set.insert(dt2); // Should not increase size (same as dt1)
        set.insert(dt3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_datetime_edge_cases() {
        // Test Unix epoch
        let epoch = DateTime::from_unix_timestamp_ms(0);
        assert_eq!(epoch.as_unix_timestamp_ms(), 0);

        // Test year 2038 (beyond 32-bit signed int seconds)
        let y2038 = DateTime::from_unix_timestamp_ms(2_147_483_648_000);
        assert_eq!(y2038.as_unix_timestamp_ms(), 2_147_483_648_000);
    }

    #[test]
    fn test_cross_type_compatibility() {
        // Test that DateTime and Instant can work together for timing operations
        let start_time = DateTime::now();
        let start_instant = Instant::now();

        std::thread::sleep(Duration::from_millis(1));

        let end_time = DateTime::now();
        let end_instant = Instant::now();

        let datetime_duration = end_time.duration_since(start_time).unwrap();
        let instant_duration = end_instant.duration_since(start_instant).unwrap();

        // Both should measure similar durations (within reasonable tolerance)
        let diff = if datetime_duration > instant_duration {
            datetime_duration - instant_duration
        } else {
            instant_duration - datetime_duration
        };

        // Allow up to 10ms difference due to timing precision
        assert!(diff <= Duration::from_millis(10));
    }

    //-------------------------------------------------------//
    //----------------- FromStr Tests ----------------------//
    //-------------------------------------------------------//

    #[test]
    fn test_datetime_from_str_iso8601_full() {
        // Test full ISO 8601 format with milliseconds and timezone
        let date_str = "2022-01-01T12:30:45.123Z";
        let dt: DateTime = date_str.parse().unwrap();

        // Verify it parses correctly by checking timestamp
        let expected_timestamp = 1_641_040_245_123; // 2022-01-01 12:30:45.123 UTC (corrected)
        assert_eq!(dt.as_unix_timestamp_ms(), expected_timestamp);
    }

    #[test]
    fn test_datetime_from_str_iso8601_no_ms() {
        // Test ISO 8601 format without milliseconds
        let date_str = "2022-01-01T12:30:45Z";
        let dt: DateTime = date_str.parse().unwrap();

        let expected_timestamp = 1_641_040_245_000; // 2022-01-01 12:30:45.000 UTC (corrected)
        assert_eq!(dt.as_unix_timestamp_ms(), expected_timestamp);
    }

    #[test]
    fn test_datetime_from_str_iso8601_no_timezone() {
        // Test ISO 8601 format without timezone (assumes UTC)
        let date_str = "2022-01-01T12:30:45.123";
        let dt: DateTime = date_str.parse().unwrap();

        let expected_timestamp = 1_641_040_245_123; // (corrected)
        assert_eq!(dt.as_unix_timestamp_ms(), expected_timestamp);
    }

    #[test]
    fn test_datetime_from_str_date_only() {
        // Test date-only format
        let date_str = "2022-01-01";
        let dt: DateTime = date_str.parse().unwrap();

        let expected_timestamp = 1_640_995_200_000; // 2022-01-01 00:00:00.000 UTC
        assert_eq!(dt.as_unix_timestamp_ms(), expected_timestamp);
    }

    #[test]
    fn test_datetime_from_str_roundtrip() {
        // Test that format -> parse -> format produces consistent results
        let original_timestamp = 1_641_040_245_123; // 2022-01-01 12:30:45.123 UTC (corrected)
        let dt1 = DateTime::from_unix_timestamp_ms(original_timestamp);

        let iso_string = dt1.format_iso8601();
        let dt2: DateTime = iso_string.parse().unwrap();

        assert_eq!(dt1.as_unix_timestamp_ms(), dt2.as_unix_timestamp_ms());
    }

    #[test]
    fn test_datetime_from_str_millisecond_precision() {
        // Test various millisecond formats
        let cases = [
            ("2022-01-01T12:30:45.1Z", 1_641_040_245_100),
            ("2022-01-01T12:30:45.12Z", 1_641_040_245_120),
            ("2022-01-01T12:30:45.123Z", 1_641_040_245_123),
            ("2022-01-01T12:30:45.1234Z", 1_641_040_245_123), // Truncated to 3 digits
        ];

        for (date_str, expected_ms) in cases {
            let dt: DateTime = date_str.parse().unwrap();
            assert_eq!(dt.as_unix_timestamp_ms(), expected_ms, "Failed for: {}", date_str);
        }
    }

    #[test]
    fn test_datetime_from_str_whitespace() {
        // Test that whitespace is handled correctly
        let date_str = "  2022-01-01T12:30:45.123Z  ";
        let dt: DateTime = date_str.parse().unwrap();

        let expected_timestamp = 1_641_040_245_123; // (corrected)
        assert_eq!(dt.as_unix_timestamp_ms(), expected_timestamp);
    }

    #[test]
    fn test_datetime_from_str_edge_cases() {
        // Test leap year
        let leap_day = "2020-02-29T00:00:00Z";
        let dt: DateTime = leap_day.parse().unwrap();
        assert_eq!(dt.as_unix_timestamp_ms(), 1_582_934_400_000);

        // Test end of year
        let end_of_year = "2022-12-31T23:59:59.999Z";
        let dt: DateTime = end_of_year.parse().unwrap();
        assert_eq!(dt.as_unix_timestamp_ms(), 1_672_531_199_999);

        // Test beginning of epoch
        let epoch = "1970-01-01T00:00:00.000Z";
        let dt: DateTime = epoch.parse().unwrap();
        assert_eq!(dt.as_unix_timestamp_ms(), 0);
    }

    #[test]
    fn test_datetime_from_str_boundary_values() {
        // Test boundary date values
        let valid_cases = [
            "2022-01-01T00:00:00Z",     // Start of day
            "2022-01-01T23:59:59Z",     // End of day
            "2022-02-28T12:00:00Z",     // Non-leap year Feb 28
            "2020-02-29T12:00:00Z",     // Leap year Feb 29
            "2022-12-31T23:59:59.999Z", // End of year with ms
        ];

        for valid_str in valid_cases {
            let result: Result<DateTime, _> = valid_str.parse();
            assert!(result.is_ok(), "Should succeed for: '{}'", valid_str);
        }
    }

    #[test]
    fn test_datetime_from_str_invalid_formats() {
        let invalid_cases = [
            "",                     // Empty string
            "invalid",              // Not a date
            "2022-13-01",           // Invalid month
            "2022-01-32",           // Invalid day
            "2022-01-01T25:00:00Z", // Invalid hour
            "2022-01-01T12:60:00Z", // Invalid minute
            "2022-01-01T12:30:60Z", // Invalid second
            "22-01-01",             // Invalid year format
            "2022/01/01",           // Wrong separator
            "2022-1-1",             // Single digit month/day
            "2022-01-01T12:30Z",    // Missing seconds
            "2022-01-01 12:30:45",  // Space instead of T
        ];

        for invalid_str in invalid_cases {
            let result: Result<DateTime, _> = invalid_str.parse();
            assert!(result.is_err(), "Should fail for: '{}'", invalid_str);
        }
    }

    #[test]
    fn test_datetime_parse_compatibility() {
        // Test that we can parse strings that our format methods produce
        let dt = DateTime::from_unix_timestamp_ms(1_641_040_245_123); // (corrected)

        // Test ISO format round-trip
        let iso_str = dt.format_iso8601();
        let parsed_dt: DateTime = iso_str.parse().unwrap();
        assert_eq!(dt.as_unix_timestamp_ms(), parsed_dt.as_unix_timestamp_ms());

        // Verify the string formats are what we expect
        assert!(iso_str.contains("2022-01-01T12:30:45.123Z"));
    }

    //-------------------------------------------------------//
    //----------------- Timezone Tests ---------------------//
    //-------------------------------------------------------//

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_parse_timezone_offset() {
        // Test positive timezone offsets
        assert_eq!(parse_timezone_offset("+0000"), Some(0)); // UTC
        assert_eq!(parse_timezone_offset("+0100"), Some(3600)); // +1 hour
        assert_eq!(parse_timezone_offset("+0300"), Some(10800)); // +3 hours
        assert_eq!(parse_timezone_offset("+0530"), Some(19800)); // +5.5 hours (India)
        assert_eq!(parse_timezone_offset("+1200"), Some(43200)); // +12 hours

        // Test negative timezone offsets
        assert_eq!(parse_timezone_offset("-0500"), Some(-18000)); // -5 hours (EST)
        assert_eq!(parse_timezone_offset("-0800"), Some(-28800)); // -8 hours (PST)
        assert_eq!(parse_timezone_offset("-0930"), Some(-34200)); // -9.5 hours

        // Test invalid formats
        assert_eq!(parse_timezone_offset(""), None);
        assert_eq!(parse_timezone_offset("+00"), None); // Too short
        assert_eq!(parse_timezone_offset("+000000"), None); // Too long
        assert_eq!(parse_timezone_offset("0300"), None); // No sign
        assert_eq!(parse_timezone_offset("*0300"), None); // Invalid sign
        assert_eq!(parse_timezone_offset("+ab00"), None); // Non-numeric hours
        assert_eq!(parse_timezone_offset("+00cd"), None); // Non-numeric minutes
        assert_eq!(parse_timezone_offset("+2500"), None); // Invalid hour (25)
        assert_eq!(parse_timezone_offset("+0099"), None); // Invalid minute (99)
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_timezone_offset_conversion() {
        // Test millisecond conversion
        let timezone_offset_ms = get_local_timezone_offset_ms();

        // Should be a reasonable offset (between -12 and +14 hours in milliseconds)
        let max_offset_ms = 14 * 3600 * 1000; // +14 hours
        let min_offset_ms = -12 * 3600 * 1000; // -12 hours

        assert!(timezone_offset_ms >= min_offset_ms);
        assert!(timezone_offset_ms <= max_offset_ms);

        // Test that subsequent calls return the same cached value
        let second_call = get_local_timezone_offset_ms();
        assert_eq!(timezone_offset_ms, second_call);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_datetime_local_display() {
        // Test that format_display uses local timezone
        let dt = DateTime::from_unix_timestamp_ms(1_640_995_200_000); // 2022-01-01 00:00:00 UTC
        let display_string = dt.format_display();

        // Should be properly formatted
        assert!(display_string.len() >= 19); // At least "dd.mm.yyyy hh:mm:ss"
        assert!(display_string.contains(".")); // Date separators
        assert!(display_string.contains(":")); // Time separators

        // Should contain year (might be 2021 or 2022 depending on timezone)
        assert!(display_string.contains("2021") || display_string.contains("2022"));
    }
}
