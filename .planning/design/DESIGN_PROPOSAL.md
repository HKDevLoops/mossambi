# Mossambi Design Proposal: Modern Fluidity, Responsive Bento & System Parity

## 1. Screenshot Critique (1115-134536 & 1115-135644)

Analysis of Haris's live screenshots at standard (~1000x700) and maximized (1080p/ultrawide) sizes:

### Standard Window (1115-134536-image.png)
- **Dead Horizontal Gaps**: Quick Access cards are fixed at 96x80px and Drives at 210x64px. They sit in a left-aligned wrapped flexbox that leaves awkward trailing empty areas rather than dynamically distributing across the viewport.
- **Disconnected Recent Files**: The filename is anchored at the left edge (`x = 32px`) while the file size is anchored at the far right edge (`x = right - 12px`). Over 600px of pure empty dark void separates them with zero intermediate columns (no Date Modified, no File Type/Kind, no Parent Path).
- **Collapsed Sidebar Hierarchy**: Because the sidebar defaults to collapsed or was closed, the window completely loses its spatial anchor. There is no visible indicator of current location aside from a solitary breadcrumb.
- **Top Toolbar Asymmetry**: Seven small icons clump together on the left (hamburger, back, forward, up, reload, new folder, home) while the right side hosts a solitary view switcher with an orange segmented button, settings gear, and search bar. There is no cohesive visual weight or rhythm.
- **Dead Network Section**: "Network Locations" renders as static gray text ("No network locations found.") taking up 60px of vertical height without utility or an action button.

### Maximized Window (1115-135644-image.png)
- **Extreme Void & Information Starvation**: On full screen (1920x1080+), over 75% of the screen area is an empty dark void. Quick Access and Drives are huddled in the upper-left quadrant because layout logic lacks responsive column stretching or bento-card scaling.
- **Ergonomic Eye-Strain in Recent Files**: A user reading `package-lock.json` must visually scan across 1600+ pixels of pitch-black void to read `84 B`. This violates basic human-factors reading guidelines.
- **Transparency Bleed-Through Defect**: Under the titlebar and tab strip, underlying terminal text (`target\release on main ... mossambi.exe`) visibly bleeds through. The frosted glass effect lacks an opaque backing layer behind text and interactive controls, failing WCAG text contrast floors (measuring under 2.5:1 against desktop clutter).
- **Zero Space Utilization**: A maximized window on a large display should feel like a command deck with rich drive telemetry, recent workspaces, storage heatmaps, and a structured multi-column data table.

---

## 2. Three Design Directions

### Direction A: "Arcane Fluid" (The Arc x macOS Bento) - RECOMMENDED
- **Mood**: Modern, tactile, breathing, elegant. Combines Arc Browser's nested vertical spaces with Apple macOS fluid spring physics and a structured modern Bento-Grid dashboard.
- **Sidebar**: Arc-style floating frosted sidebar with Spaces (Personal, Work, Dev), pinned folder pills, expandable library tree, and a bottom utility tray.
- **Standard Layout (1000x700)**:
```
+--------------------------------------------------------------------------+
| (o)(o)(o) [Personal v]  [+]                                   [  Search] |
+---------+----------------------------------------------------------------+
| SPACES  | Quick Access (Bento 4-Col)                                     |
| [Home]  | +------------+ +------------+ +------------+ +---------------+ |
| [Work]  | | Desktop    | | Downloads  | | Documents  | | Pictures      | |
|         | +------------+ +------------+ +------------+ +---------------+ |
| PINNED  | Drives & Storage Telemetry                                     |
| * Code  | +--------------------------+  +------------------------------+ |
| * Media | | C: System  [====---] 75% |  | Z: Data     [==------] 25%   | |
|         | +--------------------------+  +------------------------------+ |
| VOLUMES | Recent Files (Structured Table, max-width 900px)               |
| C: [==] | Name              Modified       Kind          Size            |
| Z: [==] | package-lock.json Today, 14:02   JSON File     84 B            |
|         | Main Edit.mov     Yesterday      QuickTime     275.7 MB        |
+---------+----------------------------------------------------------------+
```
- **Maximized Layout (1920x1080)**:
```
+---------------------------------------------------------------------------------------------------+
| (o)(o)(o) [Personal v] [Work] [+]                                          [Filter] [  Search / ] |
+-----------+---------------------------------------------------------------------------------------+
| SPACES    | BENTO DASHBOARD (Max content container: 1500px centered or 4-col auto)                |
| [Personal]| +--------------------------+ +--------------------------+ +-------------------------+ |
| [Work]    | | QUICK ACCESS (8 Cards)   | | STORAGE METRICS          | | SYSTEM HEALTH           | |
| [Media]   | | [Desk] [Down] [Docs]     | | C: [=======---] 185G free| | CPU: 12%  RAM: 8.2 GB   | |
|           | | [Pics] [Mus]  [Vid]      | | Z: [===-------]  74G free| | Temp: 42 C  Indexed: OK | |
| PINNED    | +--------------------------+ +--------------------------+ +-------------------------+ |
| * Rust    | RECENT ACTIVITY & WORKSPACES (Responsive Split View)                                  |
| * Design  | +-----------------------------------------------+ +---------------------------------+ |
|           | | Recent Files (Multi-Column Grid)              | | Quick Preview / Insights        | |
| LIBRARY   | | Name            Modified       Type      Size | | package-lock.json               | |
| > Home    | | package-lock    Today 14:02    JSON      84 B | | Size: 84 B                      | |
| > Drives  | | Main Edit.mov   Yest. 18:30    Video   275 MB | | Lines: 12                       | |
| > Trash   | | mossambi.log    Today 12:15    Log       0 B  | | Path: Z:\Projects\mossambi      | |
|           | +-----------------------------------------------+ +---------------------------------+ |
+-----------+---------------------------------------------------------------------------------------+
```
- **Typography Scale**: Display 18pt/bold, Section Header 13pt/semibold, Body 12pt/regular, Metadata/Date 11pt/regular, Numbers/Sizes 10.5pt/monospace.
- **Spacing / Radii Tokens**: Base unit 8pt (4, 8, 12, 16, 24, 32px); Card radius 10px; Pill radius 6px; Menu radius 8px.
- **Transparency Rules**: Maximum 2 layers. Layer 0: Desktop backdrop blur (Windows Mica/Acrylic). Layer 1: Window base surface (88% opacity). Layer 2: Card tiles & popups (96% solid fill with 1px specular border). Text always sits on Layer 2 (100% legibility).

---

### Direction B: "Zenith Pro" (The Minimalist Precision Finder)
- **Mood**: High-density, monochrome restraint, Swiss layout grid, zero superfluous decoration. Designed for engineering workflows.
- **Sidebar**: Ultra-slim 180px sidebar, text-only or crisp 14px monochrome SF Symbols, no pill backgrounds.
- **Layout**: Strict 2-column or 3-column data grid. Standard view prioritizes directory hierarchy. Maximized view uses a 60/40 split between interactive directory tree and rich file details.
- **Typography**: Tabular figures everywhere. Monospace for sizes, dates, permissions, and git status.
- **Tokens**: 4pt/8pt grid; 4px sharp radius; 1px subtle borders (`#ffffff18`); 100% opaque surfaces in content areas (transparency restricted to window titlebar).

---

### Direction C: "Hyperion Slate" (Linux Neo-Custom x macOS Fluidity)
- **Mood**: Power-user customizer. Modular widget slots configured via Lua/Rhai scripts.
- **Sidebar**: Dual mode: 48px icon-dock (like Blender / VS Code activity bar) expanding to 220px tree on hover or shortcut.
- **Layout**: Dynamic dashboard consisting of draggable/resizable widgets: Drive Rings, Git Repos, Bookmarks, Terminal Scratchpad, and Recent Files.
- **Typography**: Modern geometric sans (Geist / Inter) paired with Fira Code.
- **Tokens**: 8pt grid, 12px pill radius, vibrant neon accent ring on keyboard navigation focus.

---

## 3. Comprehensive Palette Role Map (Semantic Tokens)

Every theme (Dark AND Light) must define all 16 semantic roles rather than reusing 2-3 colors:

| Semantic Token | Functional Role in UI | Contrast Floor (WCAG) |
|---|---|---|
| `bg` | Layer 0: Root window canvas & glass tint | N/A (behind glass) |
| `surface_1` | Layer 1: Sidebar, header toolbar, status bar | 3:1 vs text_muted |
| `surface_2` | Layer 2: Bento cards, drive meters, table alternating rows | 4.5:1 vs text_secondary |
| `surface_3` | Layer 3: Flyouts, context menus, tooltips, active modals | 7:1 vs text_primary |
| `text_primary` | Primary file names, headings, active buttons | 7:1 vs surface_2/3 |
| `text_secondary` | Modified dates, parent folder paths, secondary labels | 4.5:1 vs surface_2 |
| `text_muted` | File sizes, hotkey hints, inactive icons | 3:1 vs surface_1/2 |
| `accent` | Primary brand action, active tab pill, progress fill | 3:1 vs surface_2 |
| `accent_alt` | Secondary highlight, search matches, badge background | 3:1 vs surface_2 |
| `selection` | Row/Card selection highlight (subtle tint with accent edge)| Distinct from hover |
| `hover` | Cursor hover fill for rows, cards, and buttons | +8% Luma delta |
| `border` | Subtle outline for cards, bento tiles, and dividers | 1.5:1 vs surface_1/2 |
| `success` | Drive space healthy (>30%), clean git state, copy finished| 4.5:1 vs surface_2 |
| `warn` | Drive space warning (>85%), lock conflicts, symlinks | 4.5:1 vs surface_2 |
| `error` | Drive space critical (>95%), failed actions, delete targets | 4.5:1 vs surface_2 |
| `tag_code` | Code & script files (`.rs`, `.ts`, `.py`, `.json`, `.c`) | 3:1 badge contrast |
| `tag_media` | Video & audio files (`.mp4`, `.mov`, `.flac`, `.mp3`) | 3:1 badge contrast |
| `tag_archive` | Compressed files (`.zip`, `.tar`, `.gz`, `.7z`) | 3:1 badge contrast |
| `tag_document` | Documents (`.pdf`, `.md`, `.docx`, `.txt`) | 3:1 badge contrast |
| `tag_image` | Image files (`.png`, `.jpg`, `.svg`, `.webp`) | 3:1 badge contrast |

### Theme Mapping Table (Dark & Light Parity)

```
Theme: Rangalipi (Signature Dark)
  bg: #1e1726 | surface_1: #241c2e | surface_2: #2d223a | surface_3: #392b4a
  text_primary: #f2e9dc | text_secondary: #c7aec3 | text_muted: #8e7699
  accent: #e4584f | accent_alt: #e9a23b | selection: #4a2d48 | hover: #382946
  border: #443254 | success: #52a972 | warn: #e9a23b | error: #e4584f
  tag_code: #689d6a | tag_media: #b16286 | tag_archive: #d79921 | tag_doc: #cc241d | tag_img: #83a598

Theme: Mossambi Punch (Signature Light)
  bg: #f8f7e6 | surface_1: #f1efd8 | surface_2: #ffffff | surface_3: #ffffff
  text_primary: #2d3319 | text_secondary: #5e6643 | text_muted: #8d9472
  accent: #4a855a | accent_alt: #d97706 | selection: #e4edd3 | hover: #eef2e1
  border: #dcd9be | success: #2e7d32 | warn: #d97706 | error: #d32f2f
  tag_code: #0284c7 | tag_media: #7c3aed | tag_archive: #b45309 | tag_doc: #be123c | tag_img: #059669

Theme: VS Code Dark+ (Dark)
  bg: #1e1e1e | surface_1: #252526 | surface_2: #2d2d2d | surface_3: #383838
  text_primary: #d4d4d4 | text_secondary: #a6a6a6 | text_muted: #858585
  accent: #569cd6 | accent_alt: #dcdcaa | selection: #094771 | hover: #333333
  border: #404040 | success: #4ec9b0 | warn: #ce9178 | error: #f44747
  tag_code: #569cd6 | tag_media: #c586c0 | tag_archive: #d7ba7d | tag_doc: #9cdcfe | tag_img: #4ec9b0

Theme: Catppuccin Mocha (Dark)
  bg: #1e1e2e | surface_1: #181825 | surface_2: #313244 | surface_3: #45475a
  text_primary: #cdd6f4 | text_secondary: #a6adc8 | text_muted: #6c7086
  accent: #89b4fa | accent_alt: #f5c2e7 | selection: #585b70 | hover: #363a4f
  border: #45475a | success: #a6e3a1 | warn: #f9e2af | error: #f38ba8
  tag_code: #89b4fa | tag_media: #cba6f7 | tag_archive: #fab387 | tag_doc: #f38ba8 | tag_img: #a6e3a1

Theme: Tokyo Night (Dark)
  bg: #1a1b26 | surface_1: #16161e | surface_2: #24283b | surface_3: #2f354f
  text_primary: #c0caf5 | text_secondary: #9aa5ce | text_muted: #565f89
  accent: #7aa2f7 | accent_alt: #e0af68 | selection: #364a82 | hover: #292e42
  border: #292e42 | success: #9ece6a | warn: #e0af68 | error: #f7768e
  tag_code: #7aa2f7 | tag_media: #bb9af7 | tag_archive: #ff9e64 | tag_doc: #f7768e | tag_img: #73daca

Theme: Gruvbox Dark (Dark)
  bg: #282828 | surface_1: #1d2021 | surface_2: #3c3836 | surface_3: #504945
  text_primary: #ebdbb2 | text_secondary: #d5c4a1 | text_muted: #928374
  accent: #fe8019 | accent_alt: #fabd2f | selection: #504945 | hover: #32302f
  border: #504945 | success: #b8bb26 | warn: #fabd2f | error: #fb4934
  tag_code: #83a598 | tag_media: #d3869b | tag_archive: #fabd2f | tag_doc: #fb4934 | tag_img: #b8bb26
```

---

## 4. Responsive Rules & Layout Grid

### Breakpoint Matrix
| Breakpoint | Width (px) | Sidebar Mode | Bento Columns | Recent Files Layout | Max Content Container |
|---|---|---|---|---|---|
| Compact | < 800px | Auto-hidden / Hamburger overlay | 1 column stacked | Compact row (Name + Size only) | 100% full width |
| Medium | 800px - 1199px | 200px solid / icon dock (48px) | 2 columns bento | 3 columns (Name, Date, Size) | 100% full width |
| Wide | 1200px - 1599px | 220px fixed Arc pill | 3 columns bento | 4 columns (Name, Date, Type, Size) | 1200px centered |
| Ultrawide | >= 1600px | 240px rich spaces dock | 4 columns bento | 5 columns (+ Parent Folder Path) | 1440px centered container |

### Bento Grid Implementation Contract
- **Home View Container**: Bound within `ui.max_rect().width.min(1440.0)`. Centered horizontally on wide monitors to eliminate the 1600px line length disaster.
- **Quick Access Cards**: Dynamic sizing using `ui.available_width()`. Instead of fixed `96.0px`, cards calculate `col_w = (total_w - (n - 1) * gap) / n`, clamping between `110px` and `160px`.
- **Drives Meter**: Expand horizontally to fill bento column slots. Storage gauge rendered as a full-width continuous rounded bar with color transitions:
  - 0% - 75%: `palette.accent` or `palette.success`
  - 76% - 89%: `palette.warn`
  - 90% - 100%: `palette.error`
- **Recent Files Table**: Render as a structured table with fixed column widths:
  - Icon + Name: 40% of available row width (truncated with tooltip)
  - Date Modified: 25% (formatted human-readable, e.g. "Today, 14:02")
  - Kind / Type: 20% (pill badge with `tag_*` color)
  - Size: 15% (monospace, right-aligned)

---

## 5. Right-Click Menu Plan: Windows Parity + Apple HIG Elegance

### Shell & Registry Integration Architecture (Windows)
1. **Registry Scanner (`src/gui/context_menu.rs`)**:
   - Query `HKCR\*\OpenWithList` and `HKCR\SystemFileAssociations\<ext>\OpenWithList` for registered handler programs.
   - Query `HKCR\Directory\shell` and `HKCR\Directory\Background\shell` for explorer commands:
     - "Open with Visual Studio Code" (`code.exe "%1"`)
     - "Open in Terminal" (`wt.exe -d "%V"`)
     - "Git Bash Here" / "Git GUI Here"
     - "7-Zip" / "WinRAR" archive handlers
     - "Scan with Microsoft Defender" (`MpCmdRun.exe`)
2. **Apple HIG Menu Hierarchy**:
   - **Section 1 (Primary Action)**: Open (Default), Open With > (App sub-list + "Choose another app..."), Quick Look (Space).
   - **Section 2 (Clipboard)**: Cut (Ctrl+X), Copy (Ctrl+C), Copy Path (Ctrl+Shift+C).
   - **Section 3 (File Manipulation)**: Rename (F2), Duplicate (Ctrl+D), Compress to ZIP, Move to Trash (Delete).
   - **Section 4 (Type-Aware Contextual Verbs)**:
     - Image: Rotate Left/Right, Set as Desktop Background, Convert Format.
     - Audio/Video: Play, Transcode, Media Inspector.
     - Document/PDF: Print, Export to PDF.
     - Code/Data: Open in Editor, Format with Prettier, Open Parent in Terminal.
   - **Section 5 (Visual Tag Dots)**: Apple-style inline horizontal bar containing 7 colored circles (Red, Orange, Yellow, Green, Blue, Purple, Gray). Clicking a dot toggles the tag instantly.
   - **Section 6 (System & 3rd-Party)**: 3rd-party entries grouped under a clean "System Actions >" or inline with native icons. Properties (Alt+Enter).

---

## 6. Deep Customization Model (Linux-Grade, Clean Defaults)

### User Configuration (`MossambiConfig`)
1. **Layout Preset Selector**:
   - `ArcModern`: Floating sidebar, Bento dashboard, responsive card scaling.
   - `FinderClassic`: Flat sidebar, horizontal drive bar, compact details list.
   - `DevStudio`: High-density monospace table, Git branch chips, terminal bottom-drawer.
2. **Glass & Material Knobs**:
   - `backdrop_material`: `MicaAlt`, `Acrylic`, `Opaque`
   - `surface_opacity`: 0.70 to 1.00 (default 0.90)
   - `specular_highlight`: Off, Low, Medium, High
3. **Sidebar Preferences**:
   - `sidebar_style`: `ArcSpaces`, `ClassicTree`, `MinimalDock`
   - Section toggles: `show_spaces`, `show_favorites`, `show_drives`, `show_tools`, `show_tags`
4. **Scripting Hooks (Rhai / Lua API)**:
   - `mossambi.on_context_menu(file_path, menu)`: Register custom user scripts into the right-click menu.
   - `mossambi.on_dashboard_widget(slot)`: Render user custom metrics or project shortcuts.

---

## 7. Recommended Direction & Implementation Roadmap

**Recommendation**: Direction A ("Arcane Fluid") with Zenith Pro data density. It solves all defects identified in the screenshots: no dead void, no 1600px unreadable lines, WCAG compliant contrast with solid card backings, and rich responsive bento spaces.

### Discrete Implementation Phases (<=1 Hour Jobs)

1. **Phase 1: Semantic Palette Expansion (`src/core/pantone.rs` & `macos_theme.rs`)**
   - *Goal*: Expand `ThemePalette` and `PantoneTheme` with all 16 semantic roles (surface_1..3, selection, hover, success/warn/error, file tag colors).
   - *Acceptance*: All 13 themes provide complete color sets; contrast test passes WCAG AA on all text surfaces.

2. **Phase 2: Responsive Bento Dashboard Grid (`src/gui/explorer_view.rs`)**
   - *Goal*: Replace fixed-size Quick Access and Drives with responsive bento grid; add 1440px max-width container.
   - *Acceptance*: Cards scale proportionally on 1000px, 1440px, and 2560px without line stretching or empty void.

3. **Phase 3: Multi-Column Recent Files Table (`src/gui/explorer_view.rs`)**
   - *Goal*: Rebuild Recent Files with structured columns: Name, Date Modified, Type Badge, and Monospace Size.
   - *Acceptance*: Maximum gap between columns is bounded; table remains legible and aligned on maximized windows.

4. **Phase 4: Arc-Style Sidebar Spaces & Floating Pill Navigation (`src/gui/studio_view.rs`)**
   - *Goal*: Add Spaces switcher pill, library sections, and bottom profile/settings dock.
   - *Acceptance*: Clean, non-collapsing visual anchor; responsive collapse to 48px icon dock on small widths.

5. **Phase 5: Context Menu Windows 3rd-Party & Type-Aware Verbs (`src/gui/context_menu.rs`)**
   - *Goal*: Add registry lookup for 3rd-party shell handlers (Code, Terminal, 7-Zip) and Apple-style 7-dot tag picker.
   - *Acceptance*: Right-click displays detected third-party tools and allows one-click color tagging.
