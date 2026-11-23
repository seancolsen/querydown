use chumsky::prelude::*;

use crate::ast::*;
use crate::highlight::TokenKind;
use crate::parser::utils::*;
use crate::tokens::*;

/// The most digits that a fraction of a second can have: microseconds, the precision to which both
/// PostgreSQL and DuckDB keep a timestamp. (Past that, PostgreSQL rounds, and DuckDB truncates.)
const MAX_FRACTION_DIGITS: usize = 6;

/// Parses a date literal, e.g. `@2024-01-31`, or a timestamp literal, which is a date followed by
/// `T` and a time of day, e.g. `@2024-01-31T11:33`, `@2024-01-31T11:33:00`, or
/// `@2024-01-31T11:33:00.25`.
///
/// A date or time that doesn't exist, like `@2024-02-30` or `@2024-01-31T24:00`, is reported as an
/// error (rather than backtracked over), so that it isn't parsed as something else. So is anything
/// written directly after the literal that would otherwise be read as a term of its own, such as a
/// time zone (`@2024-01-31T11:33Z`) or a time missing its minutes (`@2024-01-31T11`).
pub fn date_or_timestamp<'src>() -> impl Psr<'src, ExprKind> {
    let two_digits = || usize_with_digit_count(2);
    let digits = || any().filter(char::is_ascii_digit).repeated().at_least(1);

    let date = usize_with_digit_count(4)
        .then_ignore(just('-'))
        .then(two_digits())
        .then_ignore(just('-'))
        .then(two_digits())
        .map(|((year, month), day)| Date { year, month, day });

    let fraction = just('.').ignore_then(digits().collect::<String>());
    let seconds = just(':').ignore_then(two_digits()).then(fraction.or_not());
    let time = just('T')
        .ignore_then(two_digits())
        .then_ignore(just(':'))
        .then(two_digits())
        .then(seconds.or_not());

    // Anything written directly after the literal, which would otherwise be read as a term of its
    // own (`@2024-01-31T11` as a date and the search term `T11`).
    let word = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
        .repeated()
        .at_least(1)
        .to_slice();
    let after_date = word.map(|word: &str| match word.starts_with('T') {
        true => Stuck::Time,
        false => Stuck::Text(word),
    });
    let after_time = choice((
        one_of("+-")
            .then(two_digits())
            .then(just(':'))
            .then(two_digits())
            .to_slice()
            .map(Stuck::Zone),
        just(':').then(digits()).to(Stuck::Time),
        word.map(|word: &str| match word.eq_ignore_ascii_case("z") {
            true => Stuck::Zone(word),
            false => Stuck::Text(word),
        }),
    ));

    let literal = date
        .then(choice((
            time.map(Some).then(after_time.or_not()),
            empty().to(None).then(after_date.or_not()),
        )))
        .validate(|(date, (time, stuck)), extra, emitter| {
            let span = extra.span();
            let mut report = |message: String| emitter.emit(Rich::custom(span, message));
            if let Err(message) = check_date(&date) {
                report(message);
            }
            let kind = match time {
                None => ExprKind::Date(date),
                Some(((hour, minute), seconds)) => {
                    let (second, fraction) = seconds.unwrap_or_default();
                    let fraction = fraction.unwrap_or_default();
                    let time = TimeOfDay {
                        hour,
                        minute,
                        second,
                        microsecond: microseconds(&fraction),
                    };
                    if let Err(message) = check_time(&time, &fraction) {
                        report(message);
                    }
                    ExprKind::Timestamp(Timestamp { date, time })
                }
            };
            if let Some(stuck) = stuck {
                report(stuck.message(&kind));
            }
            kind
        })
        .labelled(crate::labels::DATE);

    mark(TokenKind::Date, just(CONST_SIGIL).ignore_then(literal))
}

/// Something written directly after a date or timestamp literal, which is an error.
#[derive(Clone)]
enum Stuck<'src> {
    /// A time of day that isn't written as one should be, e.g. the `T11` of `@2024-01-31T11`, or
    /// the `:5` of `@2024-01-31T11:33:5`.
    Time,
    /// A time zone after a timestamp, e.g. `Z` or `+05:00`.
    Zone(&'src str),
    /// Any other letters, digits, and underscores.
    Text(&'src str),
}

impl Stuck<'_> {
    fn message(&self, literal: &ExprKind) -> String {
        match self {
            Stuck::Time => "A time of day is written as `T` followed by two digits each for hours \
                and minutes, as in `T08:30`, and optionally for seconds, as in `T08:30:15` or \
                `T08:30:15.25`"
                .to_string(),
            Stuck::Zone(zone) => {
                format!("A timestamp can't have a time zone (`{zone}`). Write it without one")
            }
            Stuck::Text(text) => {
                let what = match literal {
                    ExprKind::Timestamp(_) => "timestamp",
                    _ => "date",
                };
                format!("`{text}` can't directly follow a {what}. Separate the two with a space")
            }
        }
    }
}

/// The digits of a fraction of a second, as a whole number of microseconds. Digits beyond the sixth
/// are ignored (and reported by [`check_time`]).
fn microseconds(fraction: &str) -> u32 {
    let digits: String = fraction.chars().take(MAX_FRACTION_DIGITS).collect();
    format!("{digits:0<width$}", width = MAX_FRACTION_DIGITS)
        .parse()
        .unwrap_or(0)
}

fn check_date(date: &Date) -> Result<(), String> {
    if !(1..=12).contains(&date.month) {
        return Err(format!(
            "`{}` isn't a date, because a month is from 01 to 12",
            date.to_iso()
        ));
    }
    let days = days_in_month(date.year, date.month);
    if !(1..=days).contains(&date.day) {
        return Err(format!(
            "`{}` isn't a date, because that month has {days} days",
            date.to_iso()
        ));
    }
    Ok(())
}

fn check_time(time: &TimeOfDay, fraction: &str) -> Result<(), String> {
    let iso = format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);
    let reason = if time.hour > 23 {
        "an hour is from 00 to 23"
    } else if time.minute > 59 {
        "a minute is from 00 to 59"
    } else if time.second > 59 {
        "a second is from 00 to 59"
    } else if fraction.len() > MAX_FRACTION_DIGITS {
        "a fraction of a second can have at most 6 digits"
    } else {
        return Ok(());
    };
    Err(format!("`{iso}` isn't a time of day, because {reason}"))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use super::*;

    fn parse(source: &str) -> Result<ExprKind, Vec<String>> {
        date_or_timestamp()
            .then_ignore(end())
            .parse(source)
            .into_result()
            .map_err(|errors| errors.iter().map(|e| e.to_string()).collect())
    }

    fn timestamp(date: (u32, u32, u32), time: (u32, u32, u32, u32)) -> ExprKind {
        let (year, month, day) = date;
        let (hour, minute, second, microsecond) = time;
        ExprKind::Timestamp(Timestamp {
            date: Date { year, month, day },
            time: TimeOfDay {
                hour,
                minute,
                second,
                microsecond,
            },
        })
    }

    #[test]
    fn date() {
        assert_eq!(
            parse("@2024-02-29"),
            Ok(ExprKind::Date(Date {
                year: 2024,
                month: 2,
                day: 29
            }))
        );
    }

    #[test]
    fn timestamps() {
        let cases = [
            ("@2026-11-22T11:33", (11, 33, 0, 0)),
            ("@2026-11-22T11:33:07", (11, 33, 7, 0)),
            ("@2026-11-22T00:00:00", (0, 0, 0, 0)),
            ("@2026-11-22T23:59:59.5", (23, 59, 59, 500_000)),
            ("@2026-11-22T23:59:59.000001", (23, 59, 59, 1)),
            ("@2026-11-22T23:59:59.123456", (23, 59, 59, 123_456)),
        ];
        for (source, time) in cases {
            assert_eq!(
                parse(source),
                Ok(timestamp((2026, 11, 22), time)),
                "{source}"
            );
        }
    }

    #[test]
    fn to_iso() {
        let iso = |source: &str| match parse(source).unwrap() {
            ExprKind::Timestamp(t) => t.to_iso(),
            other => panic!("not a timestamp: {other:?}"),
        };
        assert_eq!(iso("@2026-11-22T11:33"), "2026-11-22T11:33:00");
        assert_eq!(iso("@2026-01-02T03:04:05"), "2026-01-02T03:04:05");
        assert_eq!(iso("@2026-01-02T03:04:05.250"), "2026-01-02T03:04:05.25");
        assert_eq!(
            iso("@2026-01-02T03:04:05.000001"),
            "2026-01-02T03:04:05.000001"
        );
    }

    #[test]
    fn invalid() {
        let error = |source: &str| match parse(source) {
            Ok(kind) => panic!("{source} parsed as {kind:?}"),
            Err(errors) => errors.join("\n"),
        };
        assert_eq!(
            error("@2024-13-01"),
            "`2024-13-01` isn't a date, because a month is from 01 to 12"
        );
        assert_eq!(
            error("@2023-02-29"),
            "`2023-02-29` isn't a date, because that month has 28 days"
        );
        assert_eq!(
            error("@2024-04-31T10:00"),
            "`2024-04-31` isn't a date, because that month has 30 days"
        );
        assert_eq!(
            error("@2024-01-31T24:00"),
            "`24:00:00` isn't a time of day, because an hour is from 00 to 23"
        );
        assert_eq!(
            error("@2024-01-31T10:60"),
            "`10:60:00` isn't a time of day, because a minute is from 00 to 59"
        );
        assert_eq!(
            error("@2024-01-31T10:00:60"),
            "`10:00:60` isn't a time of day, because a second is from 00 to 59"
        );
        assert_eq!(
            error("@2024-01-31T10:00:00.1234567"),
            "`10:00:00` isn't a time of day, because a fraction of a second can have at most 6 digits"
        );
    }

    #[test]
    fn stuck_to_the_literal() {
        let error = |source: &str| match parse(source) {
            Ok(kind) => panic!("{source} parsed as {kind:?}"),
            Err(errors) => errors.join("\n"),
        };
        // A time needs hours and minutes, of two digits each, and seconds of two digits.
        for source in [
            "@2024-01-31T10",
            "@2024-01-31T1:00",
            "@2024-01-31T10:0",
            "@2024-01-31T10:00:0",
            "@2024-01-31Tx",
        ] {
            assert!(
                error(source).starts_with("A time of day is written as `T`"),
                "{source}: {}",
                error(source)
            );
        }
        for (source, zone) in [
            ("@2024-01-31T10:00Z", "Z"),
            ("@2024-01-31T10:00:00z", "z"),
            ("@2024-01-31T10:00:00.5+05:30", "+05:30"),
            ("@2024-01-31T10:00-08:00", "-08:00"),
        ] {
            assert_eq!(
                error(source),
                format!("A timestamp can't have a time zone (`{zone}`). Write it without one")
            );
        }
        assert_eq!(
            error("@2024-01-31x"),
            "`x` can't directly follow a date. Separate the two with a space"
        );
        assert_eq!(
            error("@2024-01-31T10:00UTC"),
            "`UTC` can't directly follow a timestamp. Separate the two with a space"
        );
    }
}
