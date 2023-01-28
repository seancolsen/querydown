use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use chumsky::prelude::*;

use crate::syntax_tree::*;
use crate::tokens::*;

use super::utils::*;

pub fn duration() -> impl LqlParser<Duration> {
    let part = |sym: char| positive_float().then_ignore(just(sym));
    let large_part = choice((
        part('Y').map(|value| Part { kind: Year, value }),
        part('M').map(|value| Part { kind: Month, value }),
        part('W').map(|value| Part { kind: Week, value }),
        part('D').map(|value| Part { kind: Day, value }),
    ));
    #[rustfmt::skip]
    let small_part = choice((
        part('H').map(|value| Part { kind: Hour, value }),
        part('M').map(|value| Part { kind: Minute, value }),
        part('S').map(|value| Part { kind: Second, value }),
    ));
    just(LITERAL_PREFIX).ignore_then(
        large_part
            .repeated()
            .chain::<Part, _, _>(
                just('T')
                    .ignore_then(small_part.repeated().at_least(1))
                    .or_not()
                    .flatten(),
            )
            .try_map(|v, span| assemble(v).map_err(|s| Simple::custom(span, s))),
    )
}

#[derive(Debug, PartialEq, Eq, Hash)]
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
    if parts.len() == 0 {
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
    return Ok(duration);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        let parse = |s: &str| duration().then_ignore(end()).parse(s);
        assert_eq!(
            parse("@1Y2.2M0.3W4444DT5H6M7S"),
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
        assert_eq!(parse("@0Y"), Ok(Duration::default()));
        assert_eq!(parse("@T0S"), Ok(Duration::default()));
        assert!(parse("@1M").is_ok());
        assert!(parse("@T1M").is_ok());
        assert!(parse("@1MT1M").is_ok());
        assert!(parse("@").is_err());
        assert!(parse("@1").is_err());
        assert!(parse("@1YY").is_err());
        assert!(parse("@1T").is_err());
        assert!(parse("@1YT").is_err());
        assert!(parse("@1YT1").is_err());
        assert!(parse("@1TM").is_err());
        assert!(parse("@1Y2Y").is_err());
        assert!(parse("@1Y0Y").is_err());
    }
}
