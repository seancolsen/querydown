//! Reconciling the time-zone-ness of operands in temporal binary operations.
//!
//! `@now` compiles to a zoned timestamp (`timestamptz`), while a `timestamp`/`datetime` column is
//! naive (zone-less). Most databases reconcile a mix of the two natively, but DuckDB built without
//! the ICU extension cannot — it has no timezone data with which to perform the implicit cast. So
//! whenever a subtraction or comparison puts a zoned and a naive temporal value on opposite sides,
//! we coerce the zoned side down to naive (see [`Dialect::coerce_temporal_to_naive`], which is the
//! identity on dialects that don't need it, keeping their output unchanged).
//!
//! The zone classification comes from static type inference (see [`super::typing::infer_type`]), so
//! it propagates through functions, arithmetic, and so on.

use crate::{schema::ValueType, sql::expr::build::func, sql::tree::SqlExpr, sql::Dialect};

/// The zone classification of an operand, derived from its inferred [`ValueType`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TemporalZone {
    /// A zoned timestamp (`timestamptz`), e.g. `@now`.
    Zoned,
    /// A naive temporal value: a `date` or a zone-less `timestamp`.
    Naive,
    /// Not a temporal value, or one we couldn't statically classify.
    NonTemporal,
}

impl TemporalZone {
    pub fn of(value_type: &ValueType) -> Self {
        if value_type.is_zoned() {
            TemporalZone::Zoned
        } else if value_type.is_temporal() {
            TemporalZone::Naive
        } else {
            TemporalZone::NonTemporal
        }
    }
}

/// The SQL for `@now`, together with its zone classification, for the active dialect. On a dialect
/// that can't operate on zoned timestamps (see [`Dialect::coerces_timestamptz`]) `@now` is coerced
/// to a naive timestamp at its source, so that all downstream arithmetic and comparison happens in
/// naive space; otherwise it stays the zoned `NOW()`. This is the single source of truth for `@now`
/// — [`super::typing::infer_type`] classifies it to match.
pub fn now_operand(dialect: &dyn Dialect) -> (SqlExpr, TemporalZone) {
    if dialect.coerces_timestamptz() {
        (
            dialect.coerce_temporal_to_naive(func::now()),
            TemporalZone::Naive,
        )
    } else {
        (func::now(), TemporalZone::Zoned)
    }
}

/// The zone `@now` resolves to on the active dialect, mirroring [`now_operand`]. Used by type
/// inference so the inferred type of `@now` matches the SQL actually emitted.
pub fn now_has_tz(dialect: &dyn Dialect) -> bool {
    !dialect.coerces_timestamptz()
}

/// Reconcile the two operands of a temporal binary operation (subtraction or comparison) so that
/// the operation is valid on `dialect`. When exactly one operand is a zoned timestamp and the other
/// is a naive temporal value, the zoned operand is coerced to naive. Every other combination — two
/// zoned, two naive, or anything non-temporal — is left untouched.
pub fn reconcile(
    left: (SqlExpr, TemporalZone),
    right: (SqlExpr, TemporalZone),
    dialect: &dyn Dialect,
) -> (SqlExpr, SqlExpr) {
    use TemporalZone::*;
    let (left_expr, left_zone) = left;
    let (right_expr, right_zone) = right;
    match (left_zone, right_zone) {
        (Zoned, Naive) => (dialect.coerce_temporal_to_naive(left_expr), right_expr),
        (Naive, Zoned) => (left_expr, dialect.coerce_temporal_to_naive(right_expr)),
        _ => (left_expr, right_expr),
    }
}
