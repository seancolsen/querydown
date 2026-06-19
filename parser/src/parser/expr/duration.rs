use core::fmt;
use std::collections::HashSet;
use std::fmt::Display;
use std::fmt::Formatter;

use chumsky::prelude::*;

use crate::ast::*;
use crate::parser::utils::*;

/// Parses a duration literal, e.g. `2y6m3d8h9min34.89s`.
///
/// A duration is a non-empty sequence of parts, where each part is a number immediately followed by
/// a unit. The units are case-insensitive:
///
/// | Unit  | Meaning |
/// | --    | --      |
/// | `y`   | years   |
/// | `m`   | months  |
/// | `w`   | weeks   |
/// | `d`   | days    |
/// | `h`   | hours   |
/// | `min` | minutes |
/// | `s`   | seconds |
///
/// Note that `m` always means months and `min` always means minutes, so the two are never
/// ambiguous. Because every part begins with a digit, durations never collide with column
/// references (which begin with a letter). A digit-initial token with no trailing unit is not a
/// duration — it falls through to the number parser — so `duration` must be tried before `number`.
pub fn duration<'src>() -> impl Psr<'src, Duration> {
    let case_insensitive =
        |uppercase: char| choice((just(uppercase), just(uppercase.to_ascii_lowercase())));

    // The minutes unit `min` must be tried before the months unit `m`; otherwise `m` would greedily
    // match the leading character of `min`. Chumsky rewinds on a failed alternative, so trying
    // `min` first is safe: e.g. for `m3d`, matching `min` consumes `m`, fails on `3`, rewinds, and
    // then `m` matches as months.
    let minutes = case_insensitive('M')
        .ignore_then(case_insensitive('I'))
        .ignore_then(case_insensitive('N'))
        .to(Minute);
    let unit = choice((
        minutes,
        case_insensitive('Y').to(Year),
        case_insensitive('M').to(Month),
        case_insensitive('W').to(Week),
        case_insensitive('D').to(Day),
        case_insensitive('H').to(Hour),
        case_insensitive('S').to(Second),
    ));

    let part = positive_float()
        .then(unit)
        .map(|(value, kind)| Part { kind, value });

    part.repeated()
        .at_least(1)
        .collect::<Vec<Part>>()
        .try_map(|parts, span| assemble(parts).map_err(|s| Rich::custom(span, s)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Year,
    Month,
    Week,
    Day,
    Hour,
    Minute,
    Second,
}
use Kind::*;

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Year => f.write_str("Year"),
            Month => f.write_str("Month"),
            Week => f.write_str("Week"),
            Day => f.write_str("Day"),
            Hour => f.write_str("Hour"),
            Minute => f.write_str("Minute"),
            Second => f.write_str("Second"),
        }
    }
}

struct Part {
    value: f64,
    kind: Kind,
}

fn assemble(parts: Vec<Part>) -> Result<Duration, String> {
    if parts.is_empty() {
        return Err("Duration must have at least one part".to_string());
    }
    let mut kinds_seen: HashSet<Kind> = HashSet::new();
    let mut duration = Duration::default();
    for part in parts {
        if kinds_seen.contains(&part.kind) {
            return Err(format!("Duration can't have two {} parts.", part.kind));
        }
        match part.kind {
            Year => duration.years = part.value,
            Month => duration.months = part.value,
            Week => duration.weeks = part.value,
            Day => duration.days = part.value,
            Hour => duration.hours = part.value,
            Minute => duration.minutes = part.value,
            Second => duration.seconds = part.value,
        }
        kinds_seen.insert(part.kind);
    }
    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        let parse = |s: &str| {
            duration()
                .then_ignore(end())
                .parse(s)
                .into_result()
                .map_err(|_| ())
        };
        assert_eq!(
            parse("1y2.2m0.3w4444d5h6min7s"),
            Ok(Duration {
                years: 1.0,
                months: 2.2,
                weeks: 0.3,
                days: 4444.0,
                hours: 5.0,
                minutes: 6.0,
                seconds: 7.0,
            })
        );
        // Units are case-insensitive, and parts may appear in any order.
        assert_eq!(
            parse("6MIN2Y"),
            Ok(Duration {
                years: 2.0,
                minutes: 6.0,
                ..Default::default()
            })
        );
        assert_eq!(parse("0y"), Ok(Duration::default()));
        assert_eq!(parse("0s"), Ok(Duration::default()));
        // `m` is months and `min` is minutes; the two are unambiguous, even adjacent.
        assert_eq!(
            parse("1m1min"),
            Ok(Duration {
                months: 1.0,
                minutes: 1.0,
                ..Default::default()
            })
        );
        assert!(parse("1m").is_ok());
        assert!(parse("1min").is_ok());
        // An empty string or a unit-less number is not a duration.
        assert!(parse("").is_err());
        assert!(parse("1").is_err());
        // A trailing or repeated unit character is invalid.
        assert!(parse("1yy").is_err());
        assert!(parse("1y2").is_err());
        // A duration can't repeat a kind.
        assert!(parse("1y2y").is_err());
        assert!(parse("1y0y").is_err());
        assert!(parse("1m1m").is_err());
    }
}
