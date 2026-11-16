//! Command-Line Launch Intent and Argument Parser
//!
//! Pure 7-bit ASCII. Parses folder navigation, file reveal, and mossambi://
//! deep links across standard Explorer command-line forms:
//! - "%1" (Explorer shell folder open substitution)
//! - "/select,<path>" or "/select, <path>" (Explorer file reveal syntax)
//! - "/select" "<path>" or "--select" "<path>"
//! - "mossambi://open?path=..." or "mossambi://reveal?path=..." (Action Center toast activation)
//! - Positional folder or file paths

use std::path::{Path, PathBuf};

/// High-level launch intention extracted from CLI arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchIntent {
    /// Navigate Explorer / view to the specified directory.
    Navigate(PathBuf),
    /// Open the parent directory and reveal/select the target item.
    Reveal { target: PathBuf, select: bool },
    /// Activated via a toast notification or protocol link.
    DeepLink { action: String, target: PathBuf },
    /// No file manager navigation intent (e.g. batch conversion, help, banner, daemon).
    None,
}

/// Parses a slice of command-line arguments into a LaunchIntent.
/// Skips the executable name (argv[0]) if multiple arguments are passed.
pub fn parse_launch_intent<S: AsRef<str>>(args: &[S]) -> LaunchIntent {
    if args.is_empty() {
        return LaunchIntent::None;
    }

    // Determine the relevant argument slice. If first argument looks like the binary path
    // and there are more arguments, inspect the trailing arguments.
    let slice: &[S] = if args.len() > 1 {
        let first = args[0].as_ref();
        if first.ends_with("mossambi")
            || first.ends_with("mossambi.exe")
            || first.contains('/')
            || first.contains('\\')
        {
            &args[1..]
        } else {
            args
        }
    } else {
        args
    };

    if slice.is_empty() {
        return LaunchIntent::None;
    }

    // Check for explicit subcommands or utility flags that preclude direct folder navigation
    let first_arg = slice[0].as_ref().trim();
    if matches!(
        first_arg,
        "--help"
            | "-h"
            | "help"
            | "--version"
            | "-V"
            | "version"
            | "--banner"
            | "banner"
            | "--hw"
            | "hw"
            | "--themes"
            | "themes"
            | "convert"
            | "archive"
            | "extract"
            | "bench"
            | "check"
            | "verify"
            | "catalogue"
            | "daemon"
            | "tui"
            | "widget"
            | "info"
            | "upload-ready"
            | "config"
            | "completions"
            | "_carapace"
            | "help-manual"
    ) {
        return LaunchIntent::None;
    }

    // CLI positional conversion grammar: `mossambi <input> <format|dest> ...` is a
    // conversion invocation, never a GUI launch intent, even when <input> already
    // exists on disk (an existing file is only a Reveal intent when passed alone).
    if slice.len() > 1 && Path::new(first_arg).is_file() {
        return LaunchIntent::None;
    }

    // 1. Check for mossambi:// deep links
    for arg in slice {
        let trimmed = arg.as_ref().trim();
        if trimmed.starts_with("mossambi://") {
            return parse_mossambi_protocol(trimmed);
        }
    }

    // 2. Check for /select syntax across all arguments
    // Forms:
    //   "/select,C:\path\to\file"
    //   "/select," "C:\path\to\file"
    //   "/select" "C:\path\to\file"
    //   "--select" "C:\path\to\file"
    for (i, arg) in slice.iter().enumerate() {
        let text = arg.as_ref().trim();

        // Single argument with comma: "/select,<path>" or "/select, <path>"
        let lower = text.to_ascii_lowercase();
        if lower.starts_with("/select,")
            || lower.starts_with("-select,")
            || lower.starts_with("--select,")
        {
            let after_comma = if let Some(idx) = text.find(',') {
                text[idx + 1..].trim()
            } else {
                ""
            };
            let cleaned = strip_surrounding_quotes(after_comma);
            if !cleaned.is_empty() {
                return LaunchIntent::Reveal {
                    target: PathBuf::from(cleaned),
                    select: true,
                };
            }
        }

        // Two arguments: ["/select,", "<path>"] or ["/select", "<path>"] or ["--select", "<path>"]
        if (lower == "/select" || lower == "/select," || lower == "--select" || lower == "-select")
            && i + 1 < slice.len()
        {
            let next_text = slice[i + 1].as_ref().trim();
            let cleaned = strip_surrounding_quotes(next_text);
            if !cleaned.is_empty() {
                return LaunchIntent::Reveal {
                    target: PathBuf::from(cleaned),
                    select: true,
                };
            }
        }
    }

    // 3. Subcommand "app" or "explorer" with optional path: `mossambi app <path>`
    if (first_arg == "app" || first_arg == "explorer") && slice.len() > 1 {
        let p = strip_surrounding_quotes(slice[1].as_ref().trim());
        if !p.is_empty() {
            return parse_single_path_intent(p);
        }
    }

    // 4. Single positional argument or "%1"
    let candidate = strip_surrounding_quotes(first_arg);
    if !candidate.is_empty() && !candidate.starts_with('-') {
        return parse_single_path_intent(candidate);
    }

    LaunchIntent::None
}

/// Parses a single raw argument string into a LaunchIntent.
pub fn parse_single_arg(arg: &str) -> LaunchIntent {
    parse_launch_intent(&[arg])
}

/// Resolves a single path string into either a Navigate or Reveal intent.
fn parse_single_path_intent(path_str: &str) -> LaunchIntent {
    // Literal "%1" placeholder passed by shell template before expansion
    if path_str == "%1" {
        return LaunchIntent::Navigate(PathBuf::from("%1"));
    }

    let p = Path::new(path_str);
    if p.is_dir() {
        LaunchIntent::Navigate(p.to_path_buf())
    } else if p.is_file() {
        LaunchIntent::Reveal {
            target: p.to_path_buf(),
            select: true,
        }
    } else {
        // Path might not exist on disk (or is a UNC/relative path not yet created)
        // If it looks like a directory (ends in slash) or has no file extension, treat as Navigate
        if path_str.ends_with('/') || path_str.ends_with('\\') || p.extension().is_none() {
            LaunchIntent::Navigate(p.to_path_buf())
        } else {
            LaunchIntent::Reveal {
                target: p.to_path_buf(),
                select: true,
            }
        }
    }
}

/// Parses a mossambi:// protocol URI into a LaunchIntent.
fn parse_mossambi_protocol(uri: &str) -> LaunchIntent {
    if let Some((kind, path)) = crate::platform::notify::parse_mossambi_url(uri) {
        match kind {
            crate::platform::notify::DeepLinkKind::Open => LaunchIntent::Navigate(path),
            crate::platform::notify::DeepLinkKind::Reveal => LaunchIntent::Reveal {
                target: path,
                select: true,
            },
        }
    } else {
        // Fallback parser if notify parser did not match
        let stripped = uri.strip_prefix("mossambi://").unwrap_or(uri);
        let (verb, rest) = match stripped.split_once('?') {
            Some((v, r)) => (v, r),
            None => (stripped, ""),
        };

        let target_path = rest
            .split('&')
            .find_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                if k == "path" {
                    crate::platform::notify::url_decode_component(v)
                } else {
                    None
                }
            })
            .map(PathBuf::from)
            .unwrap_or_default();

        match verb {
            "reveal" => LaunchIntent::Reveal {
                target: target_path,
                select: true,
            },
            "open" => LaunchIntent::Navigate(target_path),
            other => LaunchIntent::DeepLink {
                action: other.to_string(),
                target: target_path,
            },
        }
    }
}

/// Strips matching surrounding double quotes or single quotes from an argument string.
fn strip_surrounding_quotes(s: &str) -> &str {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_percent_one() {
        let intent = parse_single_arg("%1");
        assert_eq!(intent, LaunchIntent::Navigate(PathBuf::from("%1")));

        let intent_quoted = parse_single_arg("\"%1\"");
        assert_eq!(intent_quoted, LaunchIntent::Navigate(PathBuf::from("%1")));
    }

    #[test]
    fn test_parse_select_syntax() {
        // Single argument with comma
        let i1 = parse_single_arg("/select,C:\\Windows\\System32\\notepad.exe");
        assert_eq!(
            i1,
            LaunchIntent::Reveal {
                target: PathBuf::from("C:\\Windows\\System32\\notepad.exe"),
                select: true,
            }
        );

        // Comma with space
        let i2 = parse_single_arg("/select, C:\\Users\\User\\document.pdf");
        assert_eq!(
            i2,
            LaunchIntent::Reveal {
                target: PathBuf::from("C:\\Users\\User\\document.pdf"),
                select: true,
            }
        );

        // Two arguments: /select and path
        let args = ["mossambi", "/select", "D:\\Photos\\image.png"];
        let i3 = parse_launch_intent(&args);
        assert_eq!(
            i3,
            LaunchIntent::Reveal {
                target: PathBuf::from("D:\\Photos\\image.png"),
                select: true,
            }
        );

        // Case insensitivity: /SELECT,path
        let i4 = parse_single_arg("/SELECT,C:\\test.txt");
        assert_eq!(
            i4,
            LaunchIntent::Reveal {
                target: PathBuf::from("C:\\test.txt"),
                select: true,
            }
        );
    }

    #[test]
    fn test_parse_mossambi_uri() {
        let uri = "mossambi://open?path=C%3A%5CUsers%5CDocuments";
        let intent = parse_single_arg(uri);
        assert_eq!(
            intent,
            LaunchIntent::Navigate(PathBuf::from("C:\\Users\\Documents"))
        );

        let reveal_uri = "mossambi://reveal?path=C%3A%5Cfile.txt";
        let reveal_intent = parse_single_arg(reveal_uri);
        assert_eq!(
            reveal_intent,
            LaunchIntent::Reveal {
                target: PathBuf::from("C:\\file.txt"),
                select: true,
            }
        );
    }

    #[test]
    fn test_parse_subcommands_none() {
        assert_eq!(parse_single_arg("--help"), LaunchIntent::None);
        assert_eq!(parse_single_arg("--version"), LaunchIntent::None);
        assert_eq!(parse_single_arg("convert"), LaunchIntent::None);
    }
}
