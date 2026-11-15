//! Microsoft Fluent UI System Icons (Official Vector SVGs)
//!
//! Pure 7-bit ASCII. MIT Licensed from github.com/microsoft/fluentui-system-icons.

use crate::core::icon_packs::IconGlyph;

pub fn get_raw_fluent_svg(slug: &str, filled: bool) -> Option<&'static str> {
    match (slug, filled) {
        ("app_generic", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_app_generic_20_filled.svg"
        )),
        ("app_generic", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_app_generic_20_regular.svg"
        )),
        ("arrow_clockwise", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_clockwise_20_filled.svg"
        )),
        ("arrow_clockwise", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_clockwise_20_regular.svg"
        )),
        ("arrow_download", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_download_20_filled.svg"
        )),
        ("arrow_download", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_download_20_regular.svg"
        )),
        ("arrow_left", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_left_20_filled.svg"
        )),
        ("arrow_left", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_left_20_regular.svg"
        )),
        ("arrow_right", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_right_20_filled.svg"
        )),
        ("arrow_right", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_right_20_regular.svg"
        )),
        ("arrow_sort", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_sort_20_filled.svg"
        )),
        ("arrow_sort", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_sort_20_regular.svg"
        )),
        ("arrow_sync", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_sync_20_filled.svg"
        )),
        ("arrow_sync", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_sync_20_regular.svg"
        )),
        ("arrow_up", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_up_20_filled.svg"
        )),
        ("arrow_up", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_arrow_up_20_regular.svg"
        )),
        ("checkmark", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_checkmark_20_filled.svg"
        )),
        ("checkmark", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_checkmark_20_regular.svg"
        )),
        ("chevron_down", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_chevron_down_20_filled.svg"
        )),
        ("chevron_down", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_chevron_down_20_regular.svg"
        )),
        ("chevron_right", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_chevron_right_20_filled.svg"
        )),
        ("chevron_right", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_chevron_right_20_regular.svg"
        )),
        ("clipboard_paste", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_clipboard_paste_20_filled.svg"
        )),
        ("clipboard_paste", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_clipboard_paste_20_regular.svg"
        )),
        ("clock", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_clock_20_filled.svg"
        )),
        ("clock", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_clock_20_regular.svg"
        )),
        ("cloud", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_cloud_20_filled.svg"
        )),
        ("cloud", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_cloud_20_regular.svg"
        )),
        ("color", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_color_20_filled.svg"
        )),
        ("color", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_color_20_regular.svg"
        )),
        ("copy", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_copy_20_filled.svg"
        )),
        ("copy", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_copy_20_regular.svg"
        )),
        ("cut", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_cut_20_filled.svg"
        )),
        ("cut", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_cut_20_regular.svg"
        )),
        ("delete", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_delete_20_filled.svg"
        )),
        ("delete", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_delete_20_regular.svg"
        )),
        ("desktop", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_desktop_20_filled.svg"
        )),
        ("desktop", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_desktop_20_regular.svg"
        )),
        ("document", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_document_20_filled.svg"
        )),
        ("document", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_document_20_regular.svg"
        )),
        ("edit", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_edit_20_filled.svg"
        )),
        ("edit", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_edit_20_regular.svg"
        )),
        ("eye", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_eye_20_filled.svg"
        )),
        ("eye", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_eye_20_regular.svg"
        )),
        ("filter", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_filter_20_filled.svg"
        )),
        ("filter", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_filter_20_regular.svg"
        )),
        ("folder", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_20_filled.svg"
        )),
        ("folder", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_20_regular.svg"
        )),
        ("folder_add", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_add_20_filled.svg"
        )),
        ("folder_add", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_add_20_regular.svg"
        )),
        ("folder_open", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_open_20_filled.svg"
        )),
        ("folder_open", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_open_20_regular.svg"
        )),
        ("folder_zip", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_zip_20_filled.svg"
        )),
        ("folder_zip", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_folder_zip_20_regular.svg"
        )),
        ("grid", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_grid_20_filled.svg"
        )),
        ("grid", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_grid_20_regular.svg"
        )),
        ("hard_drive", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_hard_drive_20_filled.svg"
        )),
        ("hard_drive", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_hard_drive_20_regular.svg"
        )),
        ("home", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_home_20_filled.svg"
        )),
        ("home", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_home_20_regular.svg"
        )),
        ("image", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_image_20_filled.svg"
        )),
        ("image", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_image_20_regular.svg"
        )),
        ("info", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_info_20_filled.svg"
        )),
        ("info", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_info_20_regular.svg"
        )),
        ("key", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_key_20_filled.svg"
        )),
        ("key", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_key_20_regular.svg"
        )),
        ("keyboard", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_keyboard_20_filled.svg"
        )),
        ("keyboard", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_keyboard_20_regular.svg"
        )),
        ("lock_closed", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_lock_closed_20_filled.svg"
        )),
        ("lock_closed", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_lock_closed_20_regular.svg"
        )),
        ("lock_open", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_lock_open_20_filled.svg"
        )),
        ("lock_open", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_lock_open_20_regular.svg"
        )),
        ("more_horizontal", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_more_horizontal_20_filled.svg"
        )),
        ("more_horizontal", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_more_horizontal_20_regular.svg"
        )),
        ("more_vertical", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_more_vertical_20_filled.svg"
        )),
        ("more_vertical", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_more_vertical_20_regular.svg"
        )),
        ("music_note_2", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_music_note_2_20_filled.svg"
        )),
        ("music_note_2", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_music_note_2_20_regular.svg"
        )),
        ("navigation", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_navigation_20_filled.svg"
        )),
        ("navigation", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_navigation_20_regular.svg"
        )),
        ("open", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_open_20_filled.svg"
        )),
        ("open", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_open_20_regular.svg"
        )),
        ("pin", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_pin_20_filled.svg"
        )),
        ("pin", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_pin_20_regular.svg"
        )),
        ("rename", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_rename_20_filled.svg"
        )),
        ("rename", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_rename_20_regular.svg"
        )),
        ("ruler", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_ruler_20_filled.svg"
        )),
        ("ruler", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_ruler_20_regular.svg"
        )),
        ("search", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_search_20_filled.svg"
        )),
        ("search", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_search_20_regular.svg"
        )),
        ("select_all_on", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_select_all_on_20_filled.svg"
        )),
        ("select_all_on", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_select_all_on_20_regular.svg"
        )),
        ("settings", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_settings_20_filled.svg"
        )),
        ("settings", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_settings_20_regular.svg"
        )),
        ("share", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_share_20_filled.svg"
        )),
        ("share", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_share_20_regular.svg"
        )),
        ("shield", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_shield_20_filled.svg"
        )),
        ("shield", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_shield_20_regular.svg"
        )),
        ("shield_checkmark", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_shield_checkmark_20_filled.svg"
        )),
        ("shield_checkmark", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_shield_checkmark_20_regular.svg"
        )),
        ("star", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_star_20_filled.svg"
        )),
        ("star", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_star_20_regular.svg"
        )),
        ("table", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_table_20_filled.svg"
        )),
        ("table", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_table_20_regular.svg"
        )),
        ("tag", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_tag_20_filled.svg"
        )),
        ("tag", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_tag_20_regular.svg"
        )),
        ("video", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_video_20_filled.svg"
        )),
        ("video", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_video_20_regular.svg"
        )),
        ("window", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_window_20_filled.svg"
        )),
        ("window", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_window_20_regular.svg"
        )),
        ("window_console", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_window_console_20_filled.svg"
        )),
        ("window_console", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_window_console_20_regular.svg"
        )),
        ("wrench", true) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_wrench_20_filled.svg"
        )),
        ("wrench", false) => Some(include_str!(
            "../../assets/icons/fluent/ic_fluent_wrench_20_regular.svg"
        )),
        _ => None,
    }
}

/// Resolves an official Microsoft Fluent UI SVG for a given IconGlyph with custom hex color.
pub fn fluent_svg_for_glyph(glyph: IconGlyph, filled: bool, color_hex: &str) -> Option<String> {
    let slug = glyph_to_fluent_slug(glyph)?;
    let raw = get_raw_fluent_svg(slug, filled).or_else(|| get_raw_fluent_svg(slug, false))?;
    Some(colorize_fluent_svg(raw, color_hex))
}

/// Dynamically replaces the fill color in an official Fluent UI SVG.
pub fn colorize_fluent_svg(raw_svg: &str, color_hex: &str) -> String {
    let replacement = format!("fill=\"{}\"", color_hex);
    raw_svg.replace("fill=\"#212121\"", &replacement)
}

/// Maps an IconGlyph to its corresponding official Microsoft Fluent UI icon slug.
pub fn glyph_to_fluent_slug(glyph: IconGlyph) -> Option<&'static str> {
    Some(match glyph {
        IconGlyph::ActionBack => "arrow_left",
        IconGlyph::ActionForward => "arrow_right",
        IconGlyph::ActionUp => "arrow_up",
        IconGlyph::ActionRefresh => "arrow_clockwise",
        IconGlyph::ActionNewFolder => "folder_add",
        IconGlyph::ActionOpen => "open",
        IconGlyph::ActionFolderOpen => "folder_open",
        IconGlyph::ActionEdit => "edit",
        IconGlyph::ActionDelete => "delete",
        IconGlyph::ActionCopy => "copy",
        IconGlyph::ActionCut => "cut",
        IconGlyph::ActionRename => "rename",
        IconGlyph::ActionProperties => "info",
        IconGlyph::ActionTerminal => "window_console",
        IconGlyph::ActionCheck => "checkmark",
        IconGlyph::ActionSelectAll => "select_all_on",
        IconGlyph::ActionPaste => "clipboard_paste",
        IconGlyph::ActionEye => "eye",
        IconGlyph::ActionPin => "pin",
        IconGlyph::ActionShield => "shield_checkmark",
        IconGlyph::ActionCompress => "folder_zip",
        IconGlyph::ActionExtract => "folder_open",
        IconGlyph::ActionShortcut => "share",
        IconGlyph::ActionSort => "arrow_sort",
        IconGlyph::ActionFilter => "filter",
        IconGlyph::ActionMore => "more_horizontal",
        IconGlyph::ActionColumns => "navigation",
        IconGlyph::ActionList => "table",
        IconGlyph::Home => "home",
        IconGlyph::Search => "search",
        IconGlyph::Gear => "settings",
        IconGlyph::Cloud => "cloud",
        IconGlyph::Drive | IconGlyph::DriveSystem => "hard_drive",
        IconGlyph::RecycleBin => "delete",
        IconGlyph::Folder => "folder",
        IconGlyph::FolderDesktop => "desktop",
        IconGlyph::FolderDownloads => "arrow_download",
        IconGlyph::FolderDocuments => "document",
        IconGlyph::FolderPictures => "image",
        IconGlyph::FolderVideos => "video",
        IconGlyph::FolderMusic => "music_note_2",
        IconGlyph::FolderRepo | IconGlyph::FolderCode => "folder_open",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_fluent_icons_presence() {
        let back_svg = get_raw_fluent_svg("arrow_left", false);
        assert!(back_svg.is_some(), "arrow_left regular must exist");
        assert!(back_svg.unwrap().contains("<svg"));

        let back_filled = get_raw_fluent_svg("arrow_left", true);
        assert!(back_filled.is_some(), "arrow_left filled must exist");

        let folder_svg = get_raw_fluent_svg("folder", false);
        assert!(folder_svg.is_some(), "folder must exist");
    }

    #[test]
    fn test_fluent_glyph_colorization() {
        let colored = fluent_svg_for_glyph(IconGlyph::ActionBack, false, "#FF5500");
        assert!(colored.is_some());
        let s = colored.unwrap();
        assert!(s.contains("fill=\"#FF5500\""));
        assert!(!s.contains("fill=\"#212121\""));
    }
}
