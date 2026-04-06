use dns_dashboard::ingestor::parse_line;

#[test]
fn parses_full_line() {
    let line =
        "2026-04-06T12:00:00Z example.com A latency=42ms resolver=https://dns.example/dns-query";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.timestamp, "2026-04-06T12:00:00Z");
    assert_eq!(q.domain, "example.com");
    assert_eq!(q.query_type, "A");
    assert_eq!(q.latency_ms, Some(42));
    assert_eq!(q.blocked, false);
    assert_eq!(q.resolver.as_deref(), Some("https://dns.example/dns-query"));
}

#[test]
fn parses_blocked_with_latency() {
    let line = "2026-04-06T12:00:01Z ads.tracker.io AAAA BLOCKED latency=5ms";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.domain, "ads.tracker.io");
    assert_eq!(q.query_type, "AAAA");
    assert_eq!(q.blocked, true);
    assert_eq!(q.latency_ms, Some(5));
    assert!(q.resolver.is_none());
}

#[test]
fn parses_blocked_no_extras() {
    let line = "2026-04-06T12:00:02Z malware.com A BLOCKED";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.blocked, true);
    assert!(q.latency_ms.is_none());
    assert!(q.resolver.is_none());
}

#[test]
fn parses_minimal_line() {
    let line = "2026-04-06T12:00:03Z plain.com A";
    let q = parse_line(line).expect("should parse");
    assert_eq!(q.domain, "plain.com");
    assert_eq!(q.blocked, false);
    assert!(q.latency_ms.is_none());
    assert!(q.resolver.is_none());
}

#[test]
fn returns_none_for_short_lines() {
    assert!(parse_line("").is_none());
    assert!(parse_line("only_one_field").is_none());
    assert!(parse_line("ts domain").is_none()); // missing query_type
}

#[test]
fn ignores_unknown_fields() {
    let line = "2026-04-06T12:00:04Z x.com A unknown_field=foo latency=10ms";
    let q = parse_line(line).expect("should parse despite unknown field");
    assert_eq!(q.latency_ms, Some(10));
}
