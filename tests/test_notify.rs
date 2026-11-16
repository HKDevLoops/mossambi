//! Headless tests for the toast notification builders and mossambi:// deep
//! links. No toast is ever shown here; only the pure builders run.

use mossambi::platform::notify::{
    build_mossambi_url, build_toast_xml, parse_mossambi_url, should_notify, url_decode_component,
    url_encode_component, DeepLinkKind,
};
use std::path::{Path, PathBuf};

#[test]
fn xml_escapes_special_characters() {
    let xml = build_toast_xml("a & b <c> \"d\" 'e'", "2 &amp; <done>", None);
    assert!(xml.contains("a &amp; b &lt;c&gt; &quot;d&quot; &apos;e&apos;"));
    assert!(xml.contains("2 &amp;amp; &lt;done&gt;"));
    // No path: no launch attribute and no actions block.
    assert!(!xml.contains("launch="));
    assert!(!xml.contains("<actions>"));
}

#[test]
fn xml_includes_both_actions_with_path() {
    let p = PathBuf::from("C:\\Temp\\My Docs");
    let xml = build_toast_xml("Done", "2 succeeded, 0 failed.", Some(&p));
    let open = build_mossambi_url(DeepLinkKind::Open, &p);
    let reveal = build_mossambi_url(DeepLinkKind::Reveal, &p);
    assert!(xml.contains("<actions>"));
    assert!(xml.contains("content=\"Open folder\""));
    assert!(xml.contains("content=\"Show in Mossambi\""));
    assert!(xml.contains(&open));
    assert!(xml.contains(&reveal));
    assert!(xml.contains(&format!("launch=\"{}\"", open)));
}

#[test]
fn url_round_trip_spaces_and_unicode() {
    let raw = "C:\\My Docs\\caf\u{e9} \u{4e2d}\u{6587} \u{1f600}.txt";
    let encoded = url_encode_component(raw);
    // Everything non-ASCII is percent-encoded, so the wire form is ASCII.
    assert!(encoded.is_ascii());
    assert!(!encoded.contains(' '));
    assert_eq!(url_decode_component(&encoded).as_deref(), Some(raw));
    // '+' also decodes as a space.
    assert_eq!(
        url_decode_component("C%3A%5CMy+Docs").as_deref(),
        Some("C:\\My Docs")
    );
    // Malformed escapes fail cleanly.
    assert_eq!(url_decode_component("bad%2"), None);
}

#[test]
fn deep_link_parser_open_and_reveal() {
    let raw = "C:\\Temp\\A Folder\\caf\u{e9}.txt";
    let open = build_mossambi_url(DeepLinkKind::Open, Path::new(raw));
    let reveal = build_mossambi_url(DeepLinkKind::Reveal, Path::new(raw));
    assert_eq!(
        parse_mossambi_url(&open),
        Some((DeepLinkKind::Open, PathBuf::from(raw)))
    );
    assert_eq!(
        parse_mossambi_url(&reveal),
        Some((DeepLinkKind::Reveal, PathBuf::from(raw)))
    );
    // Foreign schemes, unknown verbs, and missing paths are rejected.
    assert_eq!(parse_mossambi_url("http://open?path=x"), None);
    assert_eq!(parse_mossambi_url("mossambi://bogus?path=x"), None);
    assert_eq!(parse_mossambi_url("mossambi://open"), None);
    assert_eq!(parse_mossambi_url("mossambi://open?path=%zz"), None);
}

#[test]
fn gate_suppresses_when_focused_and_quick() {
    use std::time::Duration;
    // Enabled + unfocused -> notify regardless of duration.
    assert!(should_notify(true, Some(false), None));
    assert!(should_notify(
        true,
        Some(false),
        Some(Duration::from_secs(1))
    ));
    // Enabled + focused + quick -> suppress.
    assert!(!should_notify(
        true,
        Some(true),
        Some(Duration::from_secs(5))
    ));
    // Enabled + focused + slow (> 10 s) -> notify.
    assert!(should_notify(
        true,
        Some(true),
        Some(Duration::from_secs(11))
    ));
    // Exactly 10 s is not "longer than" -> suppress.
    assert!(!should_notify(
        true,
        Some(true),
        Some(Duration::from_secs(10))
    ));
    // Disabled -> never, and unknown focus is treated conservatively.
    assert!(!should_notify(
        false,
        Some(false),
        Some(Duration::from_secs(60))
    ));
    assert!(!should_notify(true, None, Some(Duration::from_secs(1))));
}
