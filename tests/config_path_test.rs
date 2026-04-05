// This test verifies that Config::config_path() returns a path
// ending in "doh-proxy/config.toml" — the shape is what matters.
#[test]
fn config_path_ends_with_expected_suffix() {
    let path = doh_proxy::config::config_path();
    let s = path.to_string_lossy();
    assert!(s.ends_with("doh-proxy/config.toml"), "got: {s}");
}
