#[cfg(test)]
pub fn get_test_resource(name: &str) -> String {
    let mut d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test");
    d.push(name);
    // We unwrap here because we only ever expect this fn to run within a unit test
    std::fs::read_to_string(d).unwrap()
}
