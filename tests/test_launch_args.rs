//! Headless tests for launch-argument parsing and single-instance IPC
//! line encoding. Pure 7-bit ASCII.

use mossambi::platform::args::{parse_launch_intent, LaunchIntent};
use mossambi::platform::single_instance::{deserialize_intent, serialize_intent};
use std::path::PathBuf;

fn temp_fixture(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("mossambi_launch_args_test_{name}"))
}

#[test]
fn test_empty_args_is_none() {
    let empty: Vec<String> = Vec::new();
    assert_eq!(parse_launch_intent(&empty), LaunchIntent::None);
}

#[test]
fn test_plain_folder_navigates() {
    let dir = temp_fixture("folder");
    std::fs::create_dir_all(&dir).expect("create fixture dir");
    let args = vec![
        "mossambi.exe".to_string(),
        dir.to_string_lossy().to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Navigate(dir.clone())
    );
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn test_plain_file_reveals() {
    let file = temp_fixture("note.txt");
    std::fs::write(&file, b"x").expect("create fixture file");
    let args = vec![
        "mossambi.exe".to_string(),
        file.to_string_lossy().to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Reveal {
            target: file.clone(),
            select: true,
        }
    );
    let _ = std::fs::remove_file(&file);
}

#[test]
fn test_select_forms_reveal() {
    // Single argument with comma: "/select,<path>"
    let args = vec![
        "mossambi.exe".to_string(),
        "/select,C:\\a b\\c.txt".to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\a b\\c.txt"),
            select: true,
        }
    );

    // Split form: "/select" "<path>"
    let args = vec![
        "mossambi.exe".to_string(),
        "/select".to_string(),
        "D:\\Docs\\report.pdf".to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Reveal {
            target: PathBuf::from("D:\\Docs\\report.pdf"),
            select: true,
        }
    );

    // Split form with trailing comma: "/select," "<path>"
    let args = vec![
        "mossambi.exe".to_string(),
        "/select,".to_string(),
        "E:\\Music\\song.flac".to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Reveal {
            target: PathBuf::from("E:\\Music\\song.flac"),
            select: true,
        }
    );
}

#[test]
fn test_mossambi_uri_forms() {
    // open -> navigate to the decoded path
    let args = vec![
        "mossambi.exe".to_string(),
        "mossambi://open?path=C%3A%5CUsers%5CDoc".to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Navigate(PathBuf::from("C:\\Users\\Doc"))
    );

    // reveal -> open parent and select the decoded path
    let args = vec![
        "mossambi.exe".to_string(),
        "mossambi://reveal?path=C%3A%5Cf.txt".to_string(),
    ];
    assert_eq!(
        parse_launch_intent(&args),
        LaunchIntent::Reveal {
            target: PathBuf::from("C:\\f.txt"),
            select: true,
        }
    );
}

#[test]
fn test_ipc_line_round_trip() {
    let intents = vec![
        LaunchIntent::Navigate(PathBuf::from("C:\\Program Files\\Mossambi")),
        LaunchIntent::Reveal {
            target: PathBuf::from("D:\\Data\\Report.pdf"),
            select: true,
        },
        LaunchIntent::DeepLink {
            action: "open".to_string(),
            target: PathBuf::from("C:\\Users\\Default"),
        },
        LaunchIntent::None,
    ];
    for intent in intents {
        let line = serialize_intent(&intent);
        assert!(
            line.ends_with('\n'),
            "protocol line must be newline-terminated"
        );
        assert_eq!(deserialize_intent(&line), Some(intent));
    }

    // Garbage input decodes to nothing instead of panicking.
    assert_eq!(deserialize_intent(""), None);
    assert_eq!(deserialize_intent("\n"), None);
    assert_eq!(deserialize_intent("BOGUS\tx\n"), None);
}
