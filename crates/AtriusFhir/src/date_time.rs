use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;
use chrono::{NaiveDate, NaiveTime, DateTime as ChronoDateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use atrius_fhirpath_support::evaluation_result::EvaluationResult;
use atrius_fhirpath_support::traits::IntoEvaluationResult;
use atrius_fhirpath_support::type_info::TypeInfoResult;

/// Precision levels for FHIR Date values.
///
/// FHIR dates support partial precision, allowing year-only, year-month,
/// or full date specifications. This enum tracks which components are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatePrecision {
    /// Year only (YYYY)
    Year,
    /// Year and month (YYYY-MM)
    YearMonth,
    /// Full date (YYYY-MM-DD)
    Full,
}

/// Precision levels for FHIR Time values.
///
/// FHIR times support partial precision from hour-only through
/// sub-second precision. This enum tracks which components are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimePrecision {
    /// Hour only (HH)
    Hour,
    /// Hour and minute (HH:MM)
    HourMinute,
    /// Hour, minute, and second (HH:MM:SS)
    HourMinuteSecond,
    /// Full time with sub-second precision (HH:MM:SS.sss)
    Millisecond,
}
/// Precision levels for FHIR DateTime values.
///
/// FHIR datetimes support partial precision from year-only through
/// sub-second precision with optional timezone information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DateTimePrecision {
    /// Year only (YYYY)
    Year,
    /// Year and month (YYYY-MM)
    YearMonth,
    /// Date only (YYYY-MM-DD)
    Date,
    /// Date with hour (YYYY-MM-DDTHH)
    DateHour,
    /// Date with hour and minute (YYYY-MM-DDTHH:MM)
    DateHourMinute,
    /// Date with time to seconds (YYYY-MM-DDTHH:MM:SS)
    DateHourMinuteSecond,
    /// Full datetime with sub-second precision (YYYY-MM-DDTHH:MM:SS.sss)
    Full,
}

impl Default for PrecisionDate {
    fn default() -> Self {
        // Default to epoch date 1970-01-01
        Self::from_ymd(1970, 1, 1)
    }
}
/// Precision-aware FHIR Date type.
///
/// This type preserves the original precision and string representation
/// of FHIR date values while providing typed access to date components.
///
/// # FHIR Date Formats
/// - `YYYY` - Year only
/// - `YYYY-MM` - Year and month
/// - `YYYY-MM-DD` - Full date
///
/// # Examples
/// ```rust
/// use helios_fhir::{PrecisionDate, DatePrecision};
///
/// // Create a year-only date
/// let year_date = PrecisionDate::from_year(2023);
/// assert_eq!(year_date.precision(), DatePrecision::Year);
/// assert_eq!(year_date.original_string(), "2023");
///
/// // Create a full date
/// let full_date = PrecisionDate::from_ymd(2023, 3, 15);
/// assert_eq!(full_date.precision(), DatePrecision::Full);
/// assert_eq!(full_date.original_string(), "2023-03-15");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecisionDate {
    /// Year component (always present)
    year: i32,
    /// Month component (1-12, None for year-only precision)
    month: Option<u32>,
    /// Day component (1-31, None for year or year-month precision)
    day: Option<u32>,
    /// Precision level of this date
    precision: DatePrecision,
    /// Original string representation
    original_string: Arc<str>,
}
impl PrecisionDate {
    /// Creates a year-only precision date.
    pub fn from_year(year: i32) -> Self {
        Self {
            year,
            month: None,
            day: None,
            precision: DatePrecision::Year,
            original_string: Arc::from(format!("{:04}", year)),
        }
    }

    /// Creates a year-month precision date.
    pub fn from_year_month(year: i32, month: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: None,
            precision: DatePrecision::YearMonth,
            original_string: Arc::from(format!("{:04}-{:02}", year, month)),
        }
    }

    /// Creates a full precision date.
    pub fn from_ymd(year: i32, month: u32, day: u32) -> Self {
        Self {
            year,
            month: Some(month),
            day: Some(day),
            precision: DatePrecision::Full,
            original_string: Arc::from(format!("{:04}-{:02}-{:02}", year, month, day)),
        }
    }

    /// Parses a FHIR date string, preserving precision.
    pub fn parse(s: &str) -> Option<Self> {
        // Remove @ prefix if present
        let s = s.strip_prefix('@').unwrap_or(s);

        let parts: Vec<&str> = s.split('-').collect();
        match parts.len() {
            1 => {
                // Year only
                let year = parts[0].parse::<i32>().ok()?;
                Some(Self {
                    year,
                    month: None,
                    day: None,
                    precision: DatePrecision::Year,
                    original_string: Arc::from(s),
                })
            }
            2 => {
                // Year-month
                let year = parts[0].parse::<i32>().ok()?;
                let month = parts[1].parse::<u32>().ok()?;
                if !(1..=12).contains(&month) {
                    return None;
                }
                Some(Self {
                    year,
                    month: Some(month),
                    day: None,
                    precision: DatePrecision::YearMonth,
                    original_string: Arc::from(s),
                })
            }
            3 => {
                // Full date
                let year = parts[0].parse::<i32>().ok()?;
                let month = parts[1].parse::<u32>().ok()?;
                let day = parts[2].parse::<u32>().ok()?;
                if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
                    return None;
                }
                Some(Self {
                    year,
                    month: Some(month),
                    day: Some(day),
                    precision: DatePrecision::Full,
                    original_string: Arc::from(s),
                })
            }
            _ => None,
        }
    }

    /// Returns the precision level of this date.
    pub fn precision(&self) -> DatePrecision {
        self.precision
    }

    /// Returns the original string representation.
    pub fn original_string(&self) -> &str {
        &self.original_string
    }

    /// Returns the year component.
    pub fn year(&self) -> i32 {
        self.year
    }
    /// Returns the month component if present.
    pub fn month(&self) -> Option<u32> {
        self.month
    }

    /// Returns the day component if present.
    pub fn day(&self) -> Option<u32> {
        self.day
    }

    /// Converts to a NaiveDate, using defaults for missing components.
    pub fn to_naive_date(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month.unwrap_or(1), self.day.unwrap_or(1))
            .expect("Valid date components")
    }

    /// Compares two dates considering precision.
    /// Returns None if comparison is indeterminate due to precision differences.
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        // Compare years first
        match self.year.cmp(&other.year) {
            Ordering::Equal => {
                // Years are equal, check month precision
                match (self.month, other.month) {
                    (None, None) => Some(Ordering::Equal),
                    (None, Some(_)) | (Some(_), None) => {
                        // Different precisions - comparison may be indeterminate
                        // For < and > we can still determine, but for = it's indeterminate
                        None
                    }
                    (Some(m1), Some(m2)) => match m1.cmp(&m2) {
                        Ordering::Equal => {
                            // Months are equal, check day precision
                            match (self.day, other.day) {
                                (None, None) => Some(Ordering::Equal),
                                (None, Some(_)) | (Some(_), None) => {
                                    // Different precisions - indeterminate
                                    None
                                }
                                (Some(d1), Some(d2)) => Some(d1.cmp(&d2)),
                            }
                        }
                        other => Some(other),
                    },
                }
            }
            other => Some(other),
        }
    }
}

impl Default for PrecisionTime {
    fn default() -> Self {
        // Default to midnight 00:00:00
        Self::from_hms(0, 0, 0)
    }
}

/// Precision-aware FHIR Time type.
///
/// This type preserves the original precision and string representation
/// of FHIR time values. Note that FHIR times do not support timezone information.
///
/// # FHIR Time Formats
/// - `HH` - Hour only
/// - `HH:MM` - Hour and minute
/// - `HH:MM:SS` - Hour, minute, and second
/// - `HH:MM:SS.sss` - Full time with milliseconds
///
/// # Examples
/// ```rust
/// use helios_fhir::{PrecisionTime, TimePrecision};
///
/// // Create an hour-only time
/// let hour_time = PrecisionTime::from_hour(14);
/// assert_eq!(hour_time.precision(), TimePrecision::Hour);
/// assert_eq!(hour_time.original_string(), "14");
///
/// // Create a full precision time
/// let full_time = PrecisionTime::from_hms_milli(14, 30, 45, 123);
/// assert_eq!(full_time.precision(), TimePrecision::Millisecond);
/// assert_eq!(full_time.original_string(), "14:30:45.123");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecisionTime {
    /// Hour component (0-23, always present)
    hour: u32,
    /// Minute component (0-59)
    minute: Option<u32>,
    /// Second component (0-59)
    second: Option<u32>,
    /// Millisecond component (0-999)
    millisecond: Option<u32>,
    /// Precision level of this time
    precision: TimePrecision,
    /// Original string representation
    original_string: Arc<str>,
}

impl PrecisionTime {
    /// Creates an hour-only precision time.
    pub fn from_hour(hour: u32) -> Self {
        Self {
            hour,
            minute: None,
            second: None,
            millisecond: None,
            precision: TimePrecision::Hour,
            original_string: Arc::from(format!("{:02}", hour)),
        }
    }
    /// Creates an hour-minute precision time.
    pub fn from_hm(hour: u32, minute: u32) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: None,
            millisecond: None,
            precision: TimePrecision::HourMinute,
            original_string: Arc::from(format!("{:02}:{:02}", hour, minute)),
        }
    }

    /// Creates an hour-minute-second precision time.
    pub fn from_hms(hour: u32, minute: u32, second: u32) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: Some(second),
            millisecond: None,
            precision: TimePrecision::HourMinuteSecond,
            original_string: Arc::from(format!("{:02}:{:02}:{:02}", hour, minute, second)),
        }
    }

    /// Creates a full precision time with milliseconds.
    pub fn from_hms_milli(hour: u32, minute: u32, second: u32, millisecond: u32) -> Self {
        Self {
            hour,
            minute: Some(minute),
            second: Some(second),
            millisecond: Some(millisecond),
            precision: TimePrecision::Millisecond,
            original_string: Arc::from(format!(
                "{:02}:{:02}:{:02}.{:03}",
                hour, minute, second, millisecond
            )),
        }
    }

    /// Parses a FHIR time string, preserving precision.
    pub fn parse(s: &str) -> Option<Self> {
        // Remove @ and T prefixes if present
        let s = s.strip_prefix('@').unwrap_or(s);
        let s = s.strip_prefix('T').unwrap_or(s);

        // Check for timezone (not allowed in FHIR time)
        if s.contains('+') || s.contains('-') || s.ends_with('Z') {
            return None;
        }

        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                // Hour only
                let hour = parts[0].parse::<u32>().ok()?;
                if hour > 23 {
                    return None;
                }
                Some(Self {
                    hour,
                    minute: None,
                    second: None,
                    millisecond: None,
                    precision: TimePrecision::Hour,
                    original_string: Arc::from(s),
                })
            }
            2 => {
                // Hour:minute
                let hour = parts[0].parse::<u32>().ok()?;
                let minute = parts[1].parse::<u32>().ok()?;
                if hour > 23 || minute > 59 {
                    return None;
                }
                Some(Self {
                    hour,
                    minute: Some(minute),
                    second: None,
                    millisecond: None,
                    precision: TimePrecision::HourMinute,
                    original_string: Arc::from(s),
                })
            }
            3 => {
                // Hour:minute:second[.millisecond]
                let hour = parts[0].parse::<u32>().ok()?;
                let minute = parts[1].parse::<u32>().ok()?;

                // Check for milliseconds
                let (second, millisecond, precision) = if parts[2].contains('.') {
                    let sec_parts: Vec<&str> = parts[2].split('.').collect();
                    if sec_parts.len() != 2 {
                        return None;
                    }
                    let second = sec_parts[0].parse::<u32>().ok()?;
                    // Parse milliseconds, padding or truncating as needed
                    let ms_str = sec_parts[1];
                    let ms = if ms_str.len() <= 3 {
                        // Pad with zeros if needed
                        let padded = format!("{:0<3}", ms_str);
                        padded.parse::<u32>().ok()?
                    } else {
        // Truncate to 3 digits
                        ms_str[..3].parse::<u32>().ok()?
                    };
                    (second, Some(ms), TimePrecision::Millisecond)
                } else {
                    let second = parts[2].parse::<u32>().ok()?;
                    (second, None, TimePrecision::HourMinuteSecond)
                };

                if hour > 23 || minute > 59 || second > 59 {
                    return None;
                }

                Some(Self {
                    hour,
                    minute: Some(minute),
                    second: Some(second),
                    millisecond,
                    precision,
                    original_string: Arc::from(s),
                })
            }
            _ => None,
        }
    }

    /// Returns the precision level of this time.
    pub fn precision(&self) -> TimePrecision {
        self.precision
    }

    /// Returns the original string representation.
    pub fn original_string(&self) -> &str {
        &self.original_string
    }
    /// Converts to a NaiveTime, using defaults for missing components.
    pub fn to_naive_time(&self) -> NaiveTime {
        let milli = self.millisecond.unwrap_or(0);
        let micro = milli * 1000; // Convert milliseconds to microseconds
        NaiveTime::from_hms_micro_opt(
            self.hour,
            self.minute.unwrap_or(0),
            self.second.unwrap_or(0),
            micro,
        )
            .expect("Valid time components")
    }

    /// Compares two times considering precision.
    /// Per FHIRPath spec: seconds and milliseconds are considered the same precision level
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        match self.hour.cmp(&other.hour) {
            Ordering::Equal => {
                match (self.minute, other.minute) {
                    (None, None) => Some(Ordering::Equal),
                    (None, Some(_)) | (Some(_), None) => None,
                    (Some(m1), Some(m2)) => match m1.cmp(&m2) {
                        Ordering::Equal => {
                            match (self.second, other.second) {
                                (None, None) => Some(Ordering::Equal),
                                (None, Some(_)) | (Some(_), None) => None,
                                (Some(s1), Some(s2)) => {
                                    // Per FHIRPath spec: second and millisecond precisions are
                                    // considered a single precision using decimal comparison
                                    let ms1 = self.millisecond.unwrap_or(0);
                                    let ms2 = other.millisecond.unwrap_or(0);
                                    let total1 = s1 * 1000 + ms1;
                                    let total2 = s2 * 1000 + ms2;
                                    Some(total1.cmp(&total2))
                                }
                            }
                        }
                        other => Some(other),
                    },
                }
            }
            other => Some(other),
        }
    }
}

impl Default for PrecisionDateTime {
    fn default() -> Self {
        // Default to Unix epoch 1970-01-01T00:00:00
        Self::from_date(1970, 1, 1)
    }
}

/// Precision-aware FHIR DateTime type.
///
/// This type preserves the original precision and string representation
/// of FHIR datetime values, including timezone information when present.
///
/// # FHIR DateTime Formats
/// - `YYYY` - Year only
/// - `YYYY-MM` - Year and month
/// - `YYYY-MM-DD` - Date only
/// - `YYYY-MM-DDTHH` - Date with hour
/// - `YYYY-MM-DDTHH:MM` - Date with hour and minute
/// - `YYYY-MM-DDTHH:MM:SS` - Date with time to seconds
/// - `YYYY-MM-DDTHH:MM:SS.sss` - Full datetime with milliseconds
/// - All time formats can include timezone: `Z`, `+HH:MM`, `-HH:MM`
///
/// # Examples
/// ```rust
/// use helios_fhir::{PrecisionDateTime, DateTimePrecision};
///
/// // Create a date-only datetime
/// let date_dt = PrecisionDateTime::from_date(2023, 3, 15);
/// assert_eq!(date_dt.precision(), DateTimePrecision::Date);
/// assert_eq!(date_dt.original_string(), "2023-03-15");
///
/// // Create a full datetime with timezone
/// let full_dt = PrecisionDateTime::parse("2023-03-15T14:30:45.123Z").unwrap();
/// assert_eq!(full_dt.precision(), DateTimePrecision::Full);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecisionDateTime {
    /// Date components
    pub date: PrecisionDate,
    /// Time components (if precision includes time)
    time: Option<PrecisionTime>,
    /// Timezone offset in minutes from UTC (None means local/unspecified)
    timezone_offset: Option<i32>,
    /// Precision level of this datetime
    precision: DateTimePrecision,
    /// Original string representation
    original_string: Arc<str>,
}

impl PrecisionDateTime {
    /// Creates a year-only datetime.
    pub fn from_year(year: i32) -> Self {
        let date = PrecisionDate::from_year(year);
        Self {
            original_string: date.original_string.clone(),
            date,
            time: None,
            timezone_offset: None,
            precision: DateTimePrecision::Year,
        }
    }
    /// Creates a year-month datetime.
    pub fn from_year_month(year: i32, month: u32) -> Self {
        let date = PrecisionDate::from_year_month(year, month);
        Self {
            original_string: date.original_string.clone(),
            date,
            time: None,
            timezone_offset: None,
            precision: DateTimePrecision::YearMonth,
        }
    }

    /// Creates a date-only datetime.
    pub fn from_date(year: i32, month: u32, day: u32) -> Self {
        let date = PrecisionDate::from_ymd(year, month, day);
        Self {
            original_string: date.original_string.clone(),
            date,
            time: None,
            timezone_offset: None,
            precision: DateTimePrecision::Date,
        }
    }

    /// Parses a FHIR datetime string, preserving precision and timezone.
    pub fn parse(s: &str) -> Option<Self> {
        // Remove @ prefix if present
        let s = s.strip_prefix('@').unwrap_or(s);

        // Check for 'T' separator to determine if time is present
        if let Some(t_pos) = s.find('T') {
            let date_part = &s[..t_pos];
            let time_and_tz = &s[t_pos + 1..];
            // Parse date part
            let date = PrecisionDate::parse(date_part)?;

            // Check for timezone at the end
            let (time_part, timezone_offset) = if let Some(stripped) = time_and_tz.strip_suffix('Z')
            {
                (stripped, Some(0))
            } else if let Some(plus_pos) = time_and_tz.rfind('+') {
                let tz_str = &time_and_tz[plus_pos + 1..];
                let offset = Self::parse_timezone_offset(tz_str)?;
                (&time_and_tz[..plus_pos], Some(offset))
            } else if let Some(minus_pos) = time_and_tz.rfind('-') {
                // Be careful not to confuse negative timezone with date separator
                if minus_pos > 0 && time_and_tz[..minus_pos].contains(':') {
                    let tz_str = &time_and_tz[minus_pos + 1..];
                    let offset = Self::parse_timezone_offset(tz_str)?;
                    (&time_and_tz[..minus_pos], Some(-offset))
                } else {
                    (time_and_tz, None)
                }
            } else {
                (time_and_tz, None)
            };

            // Parse time part if not empty
            let (time, precision) = if time_part.is_empty() {
                // Just "T" with no time components (partial datetime)
                (
                    None,
                    match date.precision {
                        DatePrecision::Full => DateTimePrecision::Date,
                        DatePrecision::YearMonth => DateTimePrecision::YearMonth,
                        DatePrecision::Year => DateTimePrecision::Year,
                    },
                )
            } else {
                let time = PrecisionTime::parse(time_part)?;
                let precision = match time.precision {
                    TimePrecision::Hour => DateTimePrecision::DateHour,
                    TimePrecision::HourMinute => DateTimePrecision::DateHourMinute,
                    TimePrecision::HourMinuteSecond => DateTimePrecision::DateHourMinuteSecond,
                    TimePrecision::Millisecond => DateTimePrecision::Full,
                };
                (Some(time), precision)
            };

            Some(Self {
                date,
                time,
                timezone_offset,
                precision,
                original_string: Arc::from(s),
            })
        } else {
            // No 'T' separator, just a date
            let date = PrecisionDate::parse(s)?;
            let precision = match date.precision {
                DatePrecision::Year => DateTimePrecision::Year,
                DatePrecision::YearMonth => DateTimePrecision::YearMonth,
                DatePrecision::Full => DateTimePrecision::Date,
            };

            Some(Self {
                original_string: Arc::from(s),
                date,
                time: None,
                timezone_offset: None,
                precision,
            })
        }
    }
    /// Parses a timezone offset string (e.g., "05:30") into minutes.
    fn parse_timezone_offset(s: &str) -> Option<i32> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts.len() {
            1 => {
                // Just hours
                let hours = parts[0].parse::<i32>().ok()?;
                Some(hours * 60)
            }
            2 => {
                // Hours and minutes
                let hours = parts[0].parse::<i32>().ok()?;
                let minutes = parts[1].parse::<i32>().ok()?;
                Some(hours * 60 + minutes)
            }
            _ => None,
        }
    }

    /// Creates a PrecisionDateTime from a PrecisionDate (for date to datetime conversion).
    pub fn from_precision_date(date: PrecisionDate) -> Self {
        let precision = match date.precision {
            DatePrecision::Year => DateTimePrecision::Year,
            DatePrecision::YearMonth => DateTimePrecision::YearMonth,
            DatePrecision::Full => DateTimePrecision::Date,
        };
        Self {
            original_string: date.original_string.clone(),
            date,
            time: None,
            timezone_offset: None,
            precision,
        }
    }
    /// Returns the precision level of this datetime.
    pub fn precision(&self) -> DateTimePrecision {
        self.precision
    }

    /// Returns the original string representation.
    pub fn original_string(&self) -> &str {
        &self.original_string
    }

    /// Converts to a chrono DateTime<Utc>, using defaults for missing components.
    pub fn to_chrono_datetime(&self) -> ChronoDateTime<Utc> {
        let naive_date = self.date.to_naive_date();
        let naive_time = self
            .time
            .as_ref()
            .map(|t| t.to_naive_time())
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());

        let naive_dt = naive_date.and_time(naive_time);

        // Apply timezone offset if present
        if let Some(offset_minutes) = self.timezone_offset {
            // The datetime is in local time with the given offset
            // We need to subtract the offset to get UTC
            let utc_naive = naive_dt - chrono::Duration::minutes(offset_minutes as i64);
            ChronoDateTime::<Utc>::from_naive_utc_and_offset(utc_naive, Utc)
        } else {
            // No timezone means we assume UTC
            ChronoDateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc)
        }
    }
    /// Compares two datetimes considering precision and timezones.
    pub fn compare(&self, other: &Self) -> Option<Ordering> {
        // Check if precisions are compatible
        // Per FHIRPath spec: seconds and milliseconds are the same precision
        let self_precision_normalized = match self.precision {
            DateTimePrecision::Full => DateTimePrecision::DateHourMinuteSecond,
            p => p,
        };
        let other_precision_normalized = match other.precision {
            DateTimePrecision::Full => DateTimePrecision::DateHourMinuteSecond,
            p => p,
        };

        // If precisions don't match (except for seconds/milliseconds), return None
        if self_precision_normalized != other_precision_normalized {
            // Special handling for date vs datetime with time components
            if self.time.is_none() != other.time.is_none() {
                return None;
            }
        }

        // If both have sufficient precision and timezone info, compare as full datetimes
        if self.precision >= DateTimePrecision::DateHour
            && other.precision >= DateTimePrecision::DateHour
            && self.timezone_offset.is_some()
            && other.timezone_offset.is_some()
        {
            // Convert to UTC and compare
            return Some(self.to_chrono_datetime().cmp(&other.to_chrono_datetime()));
        }

        // If one has timezone and the other doesn't, comparison is indeterminate
        if self.timezone_offset.is_some() != other.timezone_offset.is_some() {
            return None;
        }
        // Otherwise, compare components with precision awareness
        match self.date.compare(&other.date) {
            Some(Ordering::Equal) => {
                // Dates are equal at their precision level
                match (&self.time, &other.time) {
                    (None, None) => Some(Ordering::Equal),
                    (None, Some(_)) | (Some(_), None) => None, // Different precisions
                    (Some(t1), Some(t2)) => t1.compare(t2),
                }
            }
            other => other,
        }
    }
}

// === Display Implementations for Precision Types ===

impl std::fmt::Display for PrecisionDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original_string)
    }
}

impl std::fmt::Display for PrecisionDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original_string)
    }
}

impl std::fmt::Display for PrecisionTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original_string)
    }
}
// === Serde Implementations for Precision Types ===

impl Serialize for PrecisionDate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a simple string
        serializer.serialize_str(&self.original_string)
    }
}

impl<'de> Deserialize<'de> for PrecisionDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PrecisionDate::parse(&s)
            .ok_or_else(|| de::Error::custom(format!("Invalid FHIR date format: {}", s)))
    }
}

impl Serialize for PrecisionTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a simple string
        serializer.serialize_str(&self.original_string)
    }
}

impl<'de> Deserialize<'de> for PrecisionTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PrecisionTime::parse(&s)
            .ok_or_else(|| de::Error::custom(format!("Invalid FHIR time format: {}", s)))
    }
}

impl Serialize for PrecisionDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a simple string
        serializer.serialize_str(&self.original_string)
    }
}

impl<'de> Deserialize<'de> for PrecisionDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PrecisionDateTime::parse(&s)
            .ok_or_else(|| de::Error::custom(format!("Invalid FHIR datetime format: {}", s)))
    }
} // === PrecisionInstant Implementation ===

/// A FHIR instant value that preserves the original string representation and precision.
///
/// Instants in FHIR must be complete date-time values with timezone information,
/// representing a specific moment in time. This type wraps PrecisionDateTime but
/// enforces instant-specific constraints.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrecisionInstant {
    inner: PrecisionDateTime,
}

impl PrecisionInstant {
    /// Parses a FHIR instant string.
    /// Returns None if the string is not a valid instant (must have full date, time, and timezone).
    pub fn parse(s: &str) -> Option<Self> {
        // Parse as PrecisionDateTime first
        let dt = PrecisionDateTime::parse(s)?;

        // For now, accept any valid datetime as an instant
        // In strict mode, we could require timezone, but many FHIR resources
        // use instant fields without explicit timezones
        Some(PrecisionInstant { inner: dt })
    }

    /// Returns the original string representation
    pub fn original_string(&self) -> &str {
        self.inner.original_string()
    }

    /// Get the inner PrecisionDateTime
    pub fn as_datetime(&self) -> &PrecisionDateTime {
        &self.inner
    }

    /// Convert to chrono DateTime<Utc>
    pub fn to_chrono_datetime(&self) -> ChronoDateTime<Utc> {
        // PrecisionDateTime::to_chrono_datetime returns ChronoDateTime<Utc>
        self.inner.to_chrono_datetime()
    }
}

impl fmt::Display for PrecisionInstant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl Serialize for PrecisionInstant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PrecisionInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PrecisionInstant::parse(&s)
            .ok_or_else(|| de::Error::custom(format!("Invalid FHIR instant format: {}", s)))
    }
}
// === IntoEvaluationResult Implementations for Precision Types ===

impl IntoEvaluationResult for PrecisionDate {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::date(self.original_string.to_string())
    }
}

impl IntoEvaluationResult for PrecisionTime {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::time(self.original_string.to_string())
    }
}

impl IntoEvaluationResult for PrecisionDateTime {
    fn to_evaluation_result(&self) -> EvaluationResult {
        EvaluationResult::datetime(self.original_string.to_string())
    }
}

impl IntoEvaluationResult for PrecisionInstant {
    fn to_evaluation_result(&self) -> EvaluationResult {
        // Return as datetime with instant type info
        EvaluationResult::DateTime(
            self.inner.original_string.to_string(),
            Some(TypeInfoResult::new("FHIR", "instant")),
        )
    }
}

