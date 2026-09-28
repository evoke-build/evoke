//! The clock: the calendar day a relative date — `tomorrow`, `next friday`, `the 4th` — resolves against at the
//! body's door. In: the environment. Out: a `Date`. `EVOKE_TODAY` names the day when set — a flow's, a test's —
//! else it is the machine's local day, by the C library's clock.

#![expect(unsafe_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use evoke_core::Fix;
use evoke_core::calendar::Date;
use evoke_core::name::VarName;

use super::{Environment, Failure};

/// Today: `EVOKE_TODAY` as `YYYY-MM-DD` when set — a value that is no day is the failure, naming the variable —
/// else the local calendar day.
pub fn today(environment: &Environment) -> Result<Date, Failure> {
    match environment
        .get("EVOKE_TODAY")
        .filter(|text| !text.is_empty())
    {
        Some(named) => named.parse().map_err(|why: String| Failure {
            what: "reading today's date".to_owned(),
            cause: Some(format!("EVOKE_TODAY: {why}")),
            fix: Fix::ExportKey {
                var: VarName::new("EVOKE_TODAY").expect("a variable name"),
            },
        }),
        None => Ok(local().unwrap_or_else(utc)),
    }
}

/// The machine's local calendar day, through the C library, which knows the zone; none when it cannot say.
fn local() -> Option<Date> {
    // SAFETY: `time` writes the clock into `now`, a value of ours; `localtime_r` reads `now` and writes only
    // into `tm`, ours and zeroed, which is a valid `tm` in every field; neither keeps a pointer past its return.
    let (year, month, day) = unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&raw mut now);
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&raw const now, &raw mut tm).is_null() {
            return None;
        }
        (tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday)
    };
    Date::new(year, u8::try_from(month).ok()?, u8::try_from(day).ok()?)
}

/// The day in UTC, from the system clock alone: what stands in when the C library cannot say.
fn utc() -> Date {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = i32::try_from(seconds / 86_400).unwrap_or(i32::MAX);
    Date::new(1970, 1, 1)
        .expect("the epoch is a day")
        .plus(days)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn environment(today: Option<&str>) -> Environment {
        Environment(
            today
                .map(|today| ("EVOKE_TODAY".to_owned(), today.to_owned()))
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn the_variable_names_the_day() {
        let day = today(&environment(Some("2026-05-05"))).unwrap();
        assert_eq!(day.to_string(), "2026-05-05");
    }

    #[test]
    fn a_value_that_is_no_day_is_the_failure() {
        let failure = today(&environment(Some("tomorrow"))).unwrap_err();
        assert_eq!(failure.what, "reading today's date");
        assert_eq!(
            failure.cause.as_deref(),
            Some("EVOKE_TODAY: \"tomorrow\" is not a date: YYYY-MM-DD")
        );
    }

    #[test]
    fn unset_is_the_clock_s_day() {
        let day = today(&environment(None)).unwrap();
        assert!(day.year() >= 2026, "{day}");
        // The clock's day and the UTC day differ by one at most, at the zone's edge.
        let utc = utc();
        assert!(
            (day.plus(-1)..=day.plus(1)).contains(&utc),
            "{day} vs {utc}"
        );
    }
}
