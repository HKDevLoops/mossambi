# Goldfish Annotated Review Report

Status: resolved
**Target:** Mossambi Cross-Platform Subsystems (`src/core/`, `src/gui/`, `src/platform/`)
**Environment:** Windows 11 host & Kali Linux WSL (`wsl -d kali-linux`)

---

## Prioritized Findings & AI Suggestions

### [Issue 1] [egm-codebase-details] - Severity: Blocker
*   **Affected Scope:** [src/core/fs_utils.rs#L384-L393](file:///z:/Projects/mossambi/src/core/fs_utils.rs#L384-L393) (`recycle_bin_path`)
*   **Discrepancy Description:** `home_dir()` was not found in scope on non-Windows (`error[E0425]: cannot find function home_dir in this scope`).
*   **Rationale:** During earlier refactoring, the Windows Recycle Bin implementation remained intact, but the Linux FreeDesktop Trash path was left calling `home_dir()` directly without the platform import `crate::core::platform_paths::home_dir`.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:** Linux/Unix targets failed compilation.
*   ***Applied Edit:***
    *   Location: `src/core/fs_utils.rs#L384`
    *   Change: Switched calls to `crate::core::platform_paths::home_dir()`.

### [Issue 2] [egm-codebase-details] - Severity: Blocker
*   **Affected Scope:** [src/core/tags.rs#L18-L21](file:///z:/Projects/mossambi/src/core/tags.rs#L18-L21) (`TAG_CACHE` type definition)
*   **Discrepancy Description:** `PathBuf` type not found on Linux (`error[E0425]: cannot find type PathBuf in this scope`).
*   **Rationale:** `PathBuf` was imported under `#[cfg(target_os = "windows")]` while `TAG_CACHE` (introduced in commit `7ec88e8`) used `HashMap<PathBuf, Vec<String>>` unconditionally on all platforms.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:** Cross-platform compilation failed on non-Windows targets.
*   ***Applied Edit:***
    *   Location: `src/core/tags.rs#L18`
    *   Change: Added unconditional `use std::path::{Path, PathBuf};`.

### [Issue 3] [egm-implementation-readiness] - Severity: Major
*   **Affected Scope:** [src/core/special_folders.rs#L263-L270](file:///z:/Projects/mossambi/src/core/special_folders.rs#L263-L270) (`lookup_path`)
*   **Discrepancy Description:** Test `test_known_folder_path_lookup` failed on Linux (`assertion left == right failed: None == Some(RecycleBin)`).
*   **Rationale:** On Linux/POSIX, backslash `\` is not a path separator. Checking `path.file_name()` on Windows paths like `"C:\\$Recycle.Bin"` returned the entire string `"C:\\$Recycle.Bin"`, causing string match failure.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:** Special folder resolution must correctly identify Windows path strings across both native Windows and WSL/cross-platform hosts.
*   ***Applied Edit:***
    *   Location: `src/core/special_folders.rs#L263`
    *   Change: Split across both `['\\', '/']` and used `.rfind(|s| !s.is_empty())` to safely match the tail folder name on all OS platforms.

### [Issue 4] [egm-implementation-readiness] - Severity: Major
*   **Affected Scope:** [src/gui/explorer_view.rs#L14108-L14120](file:///z:/Projects/mossambi/src/gui/explorer_view.rs#L14108-L14120) (`folder_title`)
*   **Discrepancy Description:** Test `folder_title_uses_drive_label_for_roots` failed on Linux (`assertion left == right failed: "C:\\" == "C:"`).
*   **Rationale:** On Linux, `Path::new("C:\\").file_name()` returns `Some("C:\\")`, bypassing root-trimming logic designed for Windows drive letters.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:** Title formatting must strip trailing backslashes/slashes and identify drive root patterns consistently across all OS targets.
*   ***Applied Edit:***
    *   Location: `src/gui/explorer_view.rs#L14108`
    *   Change: Trimmed trailing slashes first, detected 2-character drive roots, and extracted the tail component via `.rfind()`.

### [Issue 5] [egm-style] - Severity: Nit
*   **Affected Scope:** [src/gui/context_menu.rs](file:///z:/Projects/mossambi/src/gui/context_menu.rs), [src/gui/dwm_helper.rs](file:///z:/Projects/mossambi/src/gui/dwm_helper.rs), [src/platform/single_instance.rs](file:///z:/Projects/mossambi/src/platform/single_instance.rs), [src/core/shell_icons.rs](file:///z:/Projects/mossambi/src/core/shell_icons.rs), [src/gui/macos_theme.rs](file:///z:/Projects/mossambi/src/gui/macos_theme.rs)
*   **Discrepancy Description:** Platform-specific items (`CustomMenuKind`, `choose_backdrop`, `Backdrop`, `AtomicBool`, `Ordering`, `Arc`, `SHELL_MUTEX`, `substitute_target`, `glassify`, `opacity`) raised dead code and unused variable warnings on Linux.
*   **Rationale:** Unused items clutter build logs and can trigger CI failures under `-D warnings`.
*   ***AI Suggestion:*** PROPOSE FIX
*   ***Justification:** Guarded Windows-only utilities and statics with `#[cfg(windows)]` and `#[cfg(target_os = "windows")]`.
*   ***Applied Edit:***
    *   Location: Across the respective module headers and declarations.
    *   Change: Scoped imports and statics with target configuration gates.
