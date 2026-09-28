//! The calendar: a day, a clock time, and a day as the words read it — relative, `tomorrow`, `next monday`, `the
//! 14th`, `may fifth` — resolved against the day the host hands the core at the body's door. In: the readings
//! `propose` makes, and a `Date` from the host. Out: `Date`, displayed `YYYY-MM-DD`; `Clock`, `HH:MM`; `Day`, the
//! reading on the wire and its resolution, one pure rule. The core reads no clock: the same words read the same on
//! every day, and only the body's door knows which day it is.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A calendar day, `YYYY-MM-DD`: what the host hands in, and what a relative day resolves to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Date {
    year: i32,
    month: u8,
    day: u8,
}

/// The furthest a relative day may reach: ten years either way.
const TEN_YEARS: i32 = 3653;

impl Date {
    /// A day the calendar has, in a four-digit year.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        ((1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in(month, Some(year)))
        .then_some(Self { year, month, day })
    }

    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// The day `days` on, or back.
    #[must_use]
    pub fn plus(self, days: i32) -> Self {
        from_days(to_days(self) + i64::from(days))
    }

    /// The day of the week, Monday first.
    #[must_use]
    pub fn weekday(self) -> Weekday {
        // 1970-01-01 was a Thursday; the remainder is 0 to 6.
        let index = usize::try_from((to_days(self) + 3).rem_euclid(7)).unwrap_or(0);
        Weekday::ALL[index]
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = String;

    /// Exactly `YYYY-MM-DD`, a day the calendar has.
    fn from_str(text: &str) -> Result<Self, String> {
        let malformed = || format!("\"{text}\" is not a date: YYYY-MM-DD");
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(malformed());
        }
        let number = |from: usize, to: usize| {
            let part = &text[from..to];
            part.bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| part.parse::<u32>().ok())
                .flatten()
        };
        let (Some(year), Some(month), Some(day)) = (number(0, 4), number(5, 7), number(8, 10))
        else {
            return Err(malformed());
        };
        let day = u8::try_from(day).map_err(|_| malformed())?;
        let month = u8::try_from(month).map_err(|_| malformed())?;
        Self::new(i32::try_from(year).map_err(|_| malformed())?, month, day)
            .ok_or_else(|| format!("\"{text}\" is not a day the calendar has"))
    }
}

impl TryFrom<String> for Date {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        text.parse()
    }
}

impl From<Date> for String {
    fn from(date: Date) -> Self {
        date.to_string()
    }
}

/// A day of the week, as a weekday reading names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    /// Monday is 0.
    fn index(self) -> i32 {
        Self::ALL
            .iter()
            .position(|day| *day == self)
            .and_then(|i| i32::try_from(i).ok())
            .unwrap_or(0)
    }
}

/// The word before a weekday, when one was said: `next monday`, `this friday`, `last tuesday`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Which {
    Next,
    This,
    Last,
}

/// A day as the words read it, on the wire tagged by `type`: a day offset, `tomorrow`; a weekday and the word
/// said before it; a day of the month, `the 14th`; a month and a day, with a year or not. Validated once,
/// where serde enters too: the offset within ten years, the day one to thirty-one, the month one to twelve, a
/// calendar pair a day some year has, a calendar with a year a day that year has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", try_from = "RawDay")]
pub enum Day {
    Offset {
        days: i32,
    },
    Weekday {
        weekday: Weekday,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        which: Option<Which>,
    },
    #[serde(rename = "day")]
    Nth {
        day: u8,
    },
    Calendar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        year: Option<i32>,
        month: u8,
        day: u8,
    },
}

/// The wire shape before validation.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RawDay {
    Offset {
        days: i32,
    },
    Weekday {
        weekday: Weekday,
        #[serde(default)]
        which: Option<Which>,
    },
    #[serde(rename = "day")]
    Nth {
        day: u8,
    },
    Calendar {
        #[serde(default)]
        year: Option<i32>,
        month: u8,
        day: u8,
    },
}

impl TryFrom<RawDay> for Day {
    type Error = String;

    fn try_from(raw: RawDay) -> Result<Self, String> {
        let day = match raw {
            RawDay::Offset { days } => Self::offset(days),
            RawDay::Weekday { weekday, which } => Some(Self::Weekday { weekday, which }),
            RawDay::Nth { day } => Self::nth(day),
            RawDay::Calendar { year, month, day } => Self::calendar(year, month, day),
        };
        day.ok_or_else(|| "not a day the calendar has".to_owned())
    }
}

impl Day {
    /// Today plus `days`, within ten years either way.
    #[must_use]
    pub fn offset(days: i32) -> Option<Self> {
        (days.abs() <= TEN_YEARS).then_some(Self::Offset { days })
    }

    /// A day of the month, one to thirty-one.
    #[must_use]
    pub fn nth(day: u8) -> Option<Self> {
        (1..=31).contains(&day).then_some(Self::Nth { day })
    }

    /// A month and a day some year has; with a year, a day that year has.
    #[must_use]
    pub fn calendar(year: Option<i32>, month: u8, day: u8) -> Option<Self> {
        let holds = match year {
            Some(year) => Date::new(year, month, day).is_some(),
            None => (1..=12).contains(&month) && day >= 1 && day <= days_in(month, None),
        };
        holds.then_some(Self::Calendar { year, month, day })
    }

    /// Whether the reading names one day of the calendar whatever today is: a month, a day and a year. What a
    /// body may yield, and what a branch may list; every other reading is relative.
    #[must_use]
    pub fn is_absolute(&self) -> bool {
        matches!(self, Self::Calendar { year: Some(_), .. })
    }

    /// The reading resolved against today, one pure rule. An offset is today plus its days. A weekday, bare or
    /// with `this` or `next`, is the first such day after today, one to seven days on, never today; with `last`,
    /// the first such day before it. A day of the month is this month's when today or later, else the next
    /// month that has it. A month and a day is this year's when today or later, else the next year that has it;
    /// with a year, itself.
    #[must_use]
    pub fn resolve(&self, today: Date) -> Date {
        match *self {
            Self::Offset { days } => today.plus(days),
            Self::Weekday { weekday, which } => {
                let from = today.weekday().index();
                let to = weekday.index();
                // One to seven days: the same weekday as today is a week away, never today.
                let apart = |days: i32| if days == 0 { 7 } else { days };
                let days = if which == Some(Which::Last) {
                    -apart((from - to).rem_euclid(7))
                } else {
                    apart((to - from).rem_euclid(7))
                };
                today.plus(days)
            }
            Self::Nth { day } => {
                let (mut year, mut month) = (today.year, today.month);
                if day < today.day || day > days_in(month, Some(year)) {
                    loop {
                        (year, month) = next_month(year, month);
                        if day <= days_in(month, Some(year)) {
                            break;
                        }
                    }
                }
                Date { year, month, day }
            }
            Self::Calendar {
                year: Some(year),
                month,
                day,
            } => Date { year, month, day },
            Self::Calendar {
                year: None,
                month,
                day,
            } => {
                let mut year = today.year;
                if (month, day) < (today.month, today.day) || day > days_in(month, Some(year)) {
                    loop {
                        year += 1;
                        if day <= days_in(month, Some(year)) {
                            break;
                        }
                    }
                }
                Date { year, month, day }
            }
        }
    }
}

/// The month after, and its year.
fn next_month(year: i32, month: u8) -> (i32, u8) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// A time of day on the 24-hour clock, `HH:MM`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Clock {
    hour: u8,
    minute: u8,
}

impl Clock {
    #[must_use]
    pub fn new(hour: u8, minute: u8) -> Option<Self> {
        (hour <= 23 && minute <= 59).then_some(Self { hour, minute })
    }
}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.minute)
    }
}

impl FromStr for Clock {
    type Err = String;

    /// Exactly `HH:MM`, two digits each, on the 24-hour clock.
    fn from_str(text: &str) -> Result<Self, String> {
        let malformed = || format!("\"{text}\" is not a clock time: HH:MM");
        let bytes = text.as_bytes();
        if bytes.len() != 5
            || bytes[2] != b':'
            || !bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 2 || b.is_ascii_digit())
        {
            return Err(malformed());
        }
        let part = |from: usize| text[from..from + 2].parse::<u8>().ok();
        let (Some(hour), Some(minute)) = (part(0), part(3)) else {
            return Err(malformed());
        };
        Self::new(hour, minute).ok_or_else(malformed)
    }
}

impl TryFrom<String> for Clock {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        text.parse()
    }
}

impl From<Clock> for String {
    fn from(clock: Clock) -> Self {
        clock.to_string()
    }
}

/// The days a month has: in a year, exactly; without one, the most it can have.
#[must_use]
pub fn days_in(month: u8, year: Option<i32>) -> u8 {
    match month {
        2 => match year {
            Some(year) if !leap(year) => 28,
            _ => 29,
        },
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Days since 1970-01-01, negative before it: the proleptic Gregorian calendar, by the civil-date algorithm.
#[expect(clippy::cast_sign_loss, clippy::cast_possible_wrap)]
fn to_days(date: Date) -> i64 {
    let (y, m, d) = (
        i64::from(date.year),
        i64::from(date.month),
        i64::from(date.day),
    );
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = ((m + 9) % 12) as u64;
    let doy = (153 * mp + 2) / 5 + d as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// The day `days` after 1970-01-01, by the same algorithm the other way.
#[expect(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation
)]
fn from_days(days: i64) -> Date {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe as i64 + era * 400 + i64::from(month <= 2);
    Date {
        year: year as i32,
        month,
        day,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    fn day(json: &str) -> Day {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn a_date_reads_and_prints_as_yyyy_mm_dd() {
        assert_eq!(date("2026-05-05").to_string(), "2026-05-05");
        assert_eq!(date("2026-05-05").weekday(), Weekday::Tuesday);
        assert_eq!(date("2026-09-28").weekday(), Weekday::Monday);
        assert_eq!(date("1970-01-01").weekday(), Weekday::Thursday);
        assert_eq!(date("2024-02-29").plus(366).to_string(), "2025-03-01");
        assert_eq!(date("2026-01-01").plus(-1).to_string(), "2025-12-31");
        for bad in [
            "2026-13-05",
            "2026-02-30",
            "2026-5-5",
            "26-05-05",
            "2026-05-05x",
            "0000-01-01",
            "20a6-05-05",
        ] {
            assert!(bad.parse::<Date>().is_err(), "{bad}");
        }
        assert_eq!(
            serde_json::to_string(&date("2026-05-05")).unwrap(),
            "\"2026-05-05\""
        );
    }

    #[test]
    fn a_day_resolves_by_the_rule() {
        // A Tuesday.
        let today = date("2026-05-05");
        let resolved = |json: &str| day(json).resolve(today).to_string();
        assert_eq!(resolved(r#"{"type":"offset","days":1}"#), "2026-05-06");
        assert_eq!(resolved(r#"{"type":"offset","days":0}"#), "2026-05-05");
        assert_eq!(resolved(r#"{"type":"offset","days":-1}"#), "2026-05-04");
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"monday"}"#),
            "2026-05-11"
        );
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"monday","which":"next"}"#),
            "2026-05-11"
        );
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"tuesday"}"#),
            "2026-05-12"
        );
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"wednesday","which":"this"}"#),
            "2026-05-06"
        );
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"friday","which":"last"}"#),
            "2026-05-01"
        );
        assert_eq!(
            resolved(r#"{"type":"weekday","weekday":"tuesday","which":"last"}"#),
            "2026-04-28"
        );
        assert_eq!(resolved(r#"{"type":"day","day":5}"#), "2026-05-05");
        assert_eq!(resolved(r#"{"type":"day","day":4}"#), "2026-06-04");
        assert_eq!(resolved(r#"{"type":"day","day":31}"#), "2026-05-31");
        assert_eq!(
            resolved(r#"{"type":"calendar","month":5,"day":4}"#),
            "2027-05-04"
        );
        assert_eq!(
            resolved(r#"{"type":"calendar","month":5,"day":5}"#),
            "2026-05-05"
        );
        assert_eq!(
            resolved(r#"{"type":"calendar","month":2,"day":29}"#),
            "2028-02-29"
        );
        assert_eq!(
            resolved(r#"{"type":"calendar","year":2024,"month":2,"day":29}"#),
            "2024-02-29"
        );
        assert_eq!(resolved(r#"{"type":"offset","days":365}"#), "2027-05-05");
        // The 31st on the last day of a short month: the next month that has it.
        assert_eq!(
            day(r#"{"type":"day","day":31}"#)
                .resolve(date("2026-04-30"))
                .to_string(),
            "2026-05-31"
        );
        assert_eq!(
            day(r#"{"type":"day","day":30}"#)
                .resolve(date("2026-02-01"))
                .to_string(),
            "2026-03-30"
        );
    }

    #[test]
    fn a_day_is_validated_where_serde_enters() {
        for bad in [
            r#"{"type":"offset","days":4000}"#,
            r#"{"type":"day","day":0}"#,
            r#"{"type":"day","day":32}"#,
            r#"{"type":"calendar","month":2,"day":30}"#,
            r#"{"type":"calendar","month":13,"day":1}"#,
            r#"{"type":"calendar","year":2027,"month":2,"day":29}"#,
            r#"{"type":"weekday","weekday":"funday"}"#,
        ] {
            assert!(serde_json::from_str::<Day>(bad).is_err(), "{bad}");
        }
        let calendar = day(r#"{"type":"calendar","year":2026,"month":5,"day":5}"#);
        assert!(calendar.is_absolute());
        assert!(!day(r#"{"type":"calendar","month":5,"day":5}"#).is_absolute());
        assert_eq!(
            serde_json::to_value(&calendar).unwrap(),
            serde_json::json!({ "type": "calendar", "year": 2026, "month": 5, "day": 5 })
        );
        assert_eq!(
            serde_json::to_value(day(r#"{"type":"weekday","weekday":"monday"}"#)).unwrap(),
            serde_json::json!({ "type": "weekday", "weekday": "monday" })
        );
    }

    #[test]
    fn a_clock_is_hh_mm() {
        assert_eq!("17:30".parse::<Clock>().unwrap().to_string(), "17:30");
        assert_eq!(Clock::new(0, 0).unwrap().to_string(), "00:00");
        for bad in ["24:00", "5:30", "17:60", "17-30", "1730", "17:3a"] {
            assert!(bad.parse::<Clock>().is_err(), "{bad}");
        }
        assert_eq!(
            serde_json::to_string(&Clock::new(9, 5).unwrap()).unwrap(),
            "\"09:05\""
        );
    }

    #[test]
    fn months_have_their_days() {
        assert_eq!(days_in(2, Some(2024)), 29);
        assert_eq!(days_in(2, Some(2100)), 28);
        assert_eq!(days_in(2, Some(2000)), 29);
        assert_eq!(days_in(2, None), 29);
        assert_eq!(days_in(4, None), 30);
        assert_eq!(days_in(12, None), 31);
    }
}
