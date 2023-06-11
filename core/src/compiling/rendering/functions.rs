use crate::{compiling::scope::Scope, dialects::sql};

pub const AGO: &str = "ago";
pub const FROM_NOW: &str = "from_now";
pub const MINUS: &str = "minus";
pub const PLUS: &str = "plus";
pub const TIMES: &str = "times";
pub const DIVIDE: &str = "divide";
pub const MAX: &str = "max";
pub const MIN: &str = "min";

pub fn render_composition(
    function_name: &str,
    base: &str,
    arg: Option<String>,
    _: &mut Scope,
) -> String {
    let operator = |o: &'static str| match &arg {
        None => base.to_owned(),
        Some(a) => format!("{} {} {}", base, o, a),
    };
    let sql_fn = |f: &'static str| match &arg {
        None => format!("{}({})", f, base),
        Some(a) => format!("{}({}, {})", f, base, a),
    };
    match function_name {
        PLUS => operator(sql::PLUS),
        MINUS => operator(sql::MINUS),
        TIMES => operator(sql::TIMES),
        DIVIDE => operator(sql::DIVIDE),
        AGO => format!("({} {} {})", sql::NOW, sql::MINUS, base),
        FROM_NOW => format!("({} {} {})", sql::NOW, sql::PLUS, base),
        MAX => sql_fn(sql::MAX),
        MIN => sql_fn(sql::MIN),
        // TODO give error here instead of falling through
        _ => base.to_owned(),
    }
}
