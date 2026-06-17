use std::path::PathBuf;

pub fn get_test_resource_path(name: &str) -> PathBuf {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test");
    d.push(name);
    d
}

pub fn get_test_resource(name: &str) -> String {
    // We unwrap here because we only ever expect this fn to run within a unit test
    std::fs::read_to_string(get_test_resource_path(name)).unwrap()
}
