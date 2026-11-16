//! Headless Test for Windows Shell Context Menu Integration
//!
//! Pure 7-bit ASCII. Asserts that building the shell context menu for a
//! temporary .txt file and a temporary directory yields >0 items and finds
//! standard Windows shell verbs (e.g. "open" or "properties").

use std::fs;

#[test]
fn test_shell_context_menu_temp_file_and_folder() {
    #[cfg(target_os = "windows")]
    {
        // 1. Create a temporary .txt file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("mossambi_shell_test_file.txt");
        fs::write(&test_file, b"mossambi shell test payload").expect("Failed to write test file");

        // Build shell menu for the file
        let file_menu = mossambi::platform::build_shell_menu_sync(
            std::slice::from_ref(&test_file),
            false,
            false,
        )
        .expect("Failed to build shell menu for temp .txt file");

        assert!(
            !file_menu.is_empty(),
            "Shell menu for temp .txt file must contain >0 items"
        );

        // Flatten all titles and verbs recursively
        fn collect_verbs_and_titles(
            items: &[mossambi::platform::ShellMenuItem],
            out_verbs: &mut Vec<String>,
            out_titles: &mut Vec<String>,
        ) {
            for it in items {
                if !it.verb.is_empty() {
                    out_verbs.push(it.verb.to_lowercase());
                }
                if !it.title.is_empty() {
                    let cleaned = mossambi::platform::clean_menu_title(&it.title).to_lowercase();
                    out_titles.push(cleaned);
                }
                collect_verbs_and_titles(&it.children, out_verbs, out_titles);
            }
        }

        let mut file_verbs = Vec::new();
        let mut file_titles = Vec::new();
        collect_verbs_and_titles(&file_menu, &mut file_verbs, &mut file_titles);

        let has_known_file_verb = file_verbs.iter().any(|v| {
            v.contains("open")
                || v.contains("properties")
                || v.contains("print")
                || v.contains("edit")
        }) || file_titles.iter().any(|t| {
            t.contains("open")
                || t.contains("properties")
                || t.contains("print")
                || t.contains("edit")
        });

        assert!(
            has_known_file_verb,
            "Shell menu for .txt file must find known verbs (open/properties/edit/print). Found verbs: {:?}, titles: {:?}",
            file_verbs,
            file_titles
        );

        // 2. Create a temporary folder
        let test_folder = temp_dir.join("mossambi_shell_test_folder");
        let _ = fs::create_dir_all(&test_folder);

        // Build shell menu for the folder (item selection)
        let folder_menu = mossambi::platform::build_shell_menu_sync(
            std::slice::from_ref(&test_folder),
            false,
            false,
        )
        .expect("Failed to build shell menu for temp folder");

        assert!(
            !folder_menu.is_empty(),
            "Shell menu for temp folder must contain >0 items"
        );

        let mut folder_verbs = Vec::new();
        let mut folder_titles = Vec::new();
        collect_verbs_and_titles(&folder_menu, &mut folder_verbs, &mut folder_titles);

        let has_known_folder_verb = folder_verbs
            .iter()
            .any(|v| v.contains("open") || v.contains("explore") || v.contains("properties"))
            || folder_titles
                .iter()
                .any(|t| t.contains("open") || t.contains("explore") || t.contains("properties"));

        assert!(
            has_known_folder_verb,
            "Shell menu for folder must find known verbs (open/explore/properties). Found verbs: {:?}, titles: {:?}",
            folder_verbs,
            folder_titles
        );

        // 3. Test folder background menu
        let bg_menu = mossambi::platform::build_shell_menu_sync(
            std::slice::from_ref(&test_folder),
            true,
            false,
        )
        .expect("Failed to build background shell menu for temp folder");

        assert!(
            !bg_menu.is_empty(),
            "Background shell menu for folder must contain >0 items"
        );

        // Cleanup temp files
        let _ = fs::remove_file(test_file);
        let _ = fs::remove_dir_all(test_folder);
    }
}
