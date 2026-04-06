use doh_rs::config::Config;

#[test]
fn config_defaults_parse() {
    let toml = r#"
        listen_addr = "127.0.0.1:5353"
        upstreams = ["https://1.1.1.1/dns-query"]
    "#;
    let config: Config = toml::from_str(toml).expect("config should parse");
    assert!(!config.upstreams.is_empty());
    assert!(config.cache.enabled);
}
