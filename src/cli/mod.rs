//! Mossambi command-line interface.
//!
//! The whole CLI lives in this module tree: clap argument definitions are in
//! this file, each command group has its own file, and shared output/exit-code
//! helpers live in `output.rs`. `main.rs` is a thin entry point that only
//! initializes logging and forwards the process exit code.
//!
//! Pure ASCII by contract.

pub mod archive;
pub mod batch_rename;
pub mod config_cli;
pub mod convert;
pub mod crop;
pub mod fs_ops;
pub mod help;
pub mod inspect;
pub mod launch;
pub mod open_with;
pub mod output;
pub mod preview;
pub mod queue_cli;
pub mod route;
pub mod scripts;
pub mod search;
pub mod settings;
pub mod tags_cli;
pub mod undo;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use self::output::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "mossambi",
    author = "Mossambi Systems Team",
    version = "2.1.0",
    about = "Zero-loss native desktop file converter, archive suite, Indian exam form tuner, and macOS-style Universal File Explorer",
    long_about = "Mossambi: Zero-Loss Desktop File Converter & Universal File Explorer\nWindows & Linux | x86_64 & ARM64 | 0.00% Zero-Loss Standard | Hardware Accelerated\n\nACCESS MODALITIES:\n  1. mossambi [or: mossambi app, mossambi explorer]  -> Launch macOS-styled Universal File Explorer & Studio GUI\n  2. mossambi widget                                  -> Launch Sweet Lime Resident Radial Widget overlay\n  3. mossambi tui                                     -> Launch Yazi-grade interactive Terminal File Explorer\n  4. mossambi <INPUT> [TARGET_FORMAT]                 -> Fast UNIX mv-style file conversion",
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    /// Input file or directory to process (mv-style positional grammar).
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Target format extension (e.g. 'wav', 'mp3', 'webp', 'pdf', or 'ALL').
    #[arg(value_name = "TARGET_FORMAT")]
    pub target: Option<String>,

    /// Batch convert to ALL valid formats in this category simultaneously.
    #[arg(short = 'a', long = "all")]
    pub all: bool,

    /// Indian Competitive Exam preset (e.g. upsc-photo, ssc-photo, ibps-signature).
    #[arg(long = "exam")]
    pub exam_preset: Option<String>,

    /// Exam custom target width in pixels.
    #[arg(long = "width")]
    pub exam_width: Option<u32>,

    /// Exam custom target height in pixels.
    #[arg(long = "height")]
    pub exam_height: Option<u32>,

    /// Exam custom target DPI (dots per inch).
    #[arg(long = "dpi")]
    pub exam_dpi: Option<u32>,

    /// Exam minimum file size in kilobytes.
    #[arg(long = "min-kb")]
    pub exam_min_kb: Option<u64>,

    /// Exam maximum file size in kilobytes.
    #[arg(long = "max-kb")]
    pub exam_max_kb: Option<u64>,

    /// Auto-whiten background of scanned documents or signatures.
    #[arg(long = "whiten")]
    pub exam_whiten: bool,

    /// Destination directory for output files (defaults to sibling directory).
    #[arg(short = 'o', long = "output")]
    pub output_dir: Option<PathBuf>,

    /// Enforce strict 0.00% zero-loss transcode (stream copy / lossless PCM / raster).
    #[arg(long = "strict", global = true, default_value_t = true)]
    pub strict_lossless: bool,

    /// Output results in machine-readable JSON for scripts and integrations.
    #[arg(long = "json", global = true)]
    pub json: bool,

    /// Launch the full Universal File Explorer & Conversion Studio GUI.
    #[arg(long = "app", visible_alias = "gui", visible_alias = "explorer")]
    pub app: bool,

    /// Launch the Sweet Lime Desktop Radial Widget.
    #[arg(long = "widget")]
    pub widget: bool,

    /// Launch the Terminal User Interface (TUI) File Explorer.
    #[arg(long = "tui")]
    pub tui: bool,

    /// Display the MOSSAMBI banner and summary.
    #[arg(long = "banner")]
    pub banner: bool,

    /// Display the Unicode box table manual.
    #[arg(long = "manual", visible_alias = "help-manual")]
    pub manual: bool,

    /// Display detected hardware acceleration engines.
    #[arg(long = "hw")]
    pub hw: bool,

    /// List all named Pantone themes.
    #[arg(long = "themes")]
    pub themes: bool,

    /// 4-dimension Quality & Container Integrity Scorecard.
    #[arg(long = "check")]
    pub check: Option<PathBuf>,

    /// Benchmark conversion latency and throughput.
    #[arg(long = "bench")]
    pub bench: Option<PathBuf>,

    /// Verify mathematical zero-loss fidelity between two files.
    #[arg(long = "verify", num_args = 2)]
    pub verify: Option<Vec<PathBuf>>,

    /// Search across the 200+ format registry catalog.
    #[arg(long = "catalogue")]
    pub catalogue: Option<String>,

    /// Pack files or directories into an archive.
    #[arg(long = "archive", num_args = 1..)]
    pub archive: Option<Vec<PathBuf>>,

    /// Extract an archive with automatic Zip-Slip protection.
    #[arg(long = "extract")]
    pub extract: Option<PathBuf>,

    /// Manage background resident daemon (start, stop, status).
    #[arg(long = "daemon")]
    pub daemon: Option<String>,

    /// Explicit subcommands.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Launch the Universal File Explorer & Conversion Studio GUI (default app).
    #[command(visible_alias = "gui")]
    App {
        /// Optional directory or file to open in the Explorer.
        path: Option<PathBuf>,
    },

    /// Launch the Universal File Explorer & Conversion Studio GUI.
    Explorer {
        /// Optional directory or file to open in the Explorer.
        path: Option<PathBuf>,
    },

    /// Launch the Terminal User Interface (TUI) File Explorer.
    Tui {
        /// Optional directory to open in the TUI.
        path: Option<PathBuf>,
    },

    /// Launch or summon the Sweet Lime Resident Radial Widget overlay.
    Widget {
        /// Optional X,Y spawn coordinates (e.g. 500,400).
        #[arg(long = "pos")]
        pos: Option<String>,

        /// Flag indicating widget was awakened by an active mouse drag.
        #[arg(long = "active-drag")]
        active_drag: bool,

        /// Initial file to load into the widget.
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },

    /// Convert an input file to a target format (explicit syntax).
    Convert {
        /// Source file path.
        input: PathBuf,
        /// Target format or 'ALL'.
        target: String,
        /// Custom destination directory.
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Batch convert to ALL valid formats in the domain.
        #[arg(short = 'a', long = "all")]
        all: bool,
    },

    /// Pack files or directories into an archive (ZIP, TAR, GZ, BZ2, XZ, ZST).
    Archive {
        /// Input files or folders to compress.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Output archive filename (e.g. output.tar.gz, backup.zip, bundle.zip).
        #[arg(short = 'o', long = "output", required = true)]
        output: PathBuf,
        /// Explicit archive format (optional, inferred from extension if omitted).
        #[arg(short = 't', long = "type")]
        archive_type: Option<String>,
    },

    /// Extract an archive with automatic Zip-Slip directory traversal protection.
    Extract {
        /// Path to archive file (.zip, .tar, .tar.gz, etc.).
        archive: PathBuf,
        /// Output extraction directory (defaults to sibling folder).
        #[arg(short = 'o', long = "output")]
        output_dir: Option<PathBuf>,
    },

    /// Inspect file metadata, detected category, and available target formats.
    Info {
        /// File path to inspect.
        file: PathBuf,
    },

    /// Check quality, container integrity, compatibility, and decoder parseability.
    Check {
        /// File path to inspect.
        file: PathBuf,
    },

    /// Benchmark conversion latency, throughput (MB/s), and speedup.
    Bench {
        /// Input file to benchmark.
        file: PathBuf,
        /// Optional target format to benchmark.
        target: Option<String>,
    },

    /// Display detected hardware acceleration engines.
    Hw,

    /// Verify mathematical zero-loss fidelity between two files.
    Verify {
        /// Original source file.
        source: PathBuf,
        /// Converted target file.
        target: PathBuf,
    },

    /// Search across the 200+ format registry and catalogue.
    Catalogue {
        /// Search query (e.g. 'flac', 'pdf', 'lossless', 'archive').
        #[arg(default_value = "")]
        query: String,
    },

    /// Prepare an image/document for an Indian competitive exam / job portal.
    UploadReady {
        /// Source image or document.
        input: PathBuf,
        /// Exam preset (e.g. upsc-photo, ssc-photo, ibps-document).
        #[arg(long = "preset")]
        preset: Option<String>,
        /// Target width in pixels.
        #[arg(long = "width")]
        width: Option<u32>,
        /// Target height in pixels.
        #[arg(long = "height")]
        height: Option<u32>,
        /// Target resolution DPI.
        #[arg(long = "dpi")]
        dpi: Option<u32>,
        /// Minimum file size in kilobytes.
        #[arg(long = "min-kb")]
        min_kb: Option<u64>,
        /// Maximum file size in kilobytes.
        #[arg(long = "max-kb")]
        max_kb: Option<u64>,
        /// Auto-whiten scanned signatures or photos.
        #[arg(long = "whiten")]
        whiten: bool,
        /// Custom destination directory.
        #[arg(short = 'o', long = "output")]
        output_dir: Option<PathBuf>,
    },

    /// List all named Pantone themes with codes and hex values.
    Themes,

    /// View or edit persistent configuration settings (~/.mossambi/config.json).
    Config {
        /// Action: 'get', 'set', or 'list'.
        #[arg(default_value = "list")]
        action: String,
        /// Key to view or change (e.g. 'shade', 'theme', 'always_on_top').
        key: Option<String>,
        /// New value to assign.
        value: Option<String>,
    },

    /// Generate a shell auto-completion script or export a Carapace YAML spec.
    Completions {
        /// Target shell: bash, zsh, fish, powershell, elvish. (Auto-detected if omitted)
        shell: Option<String>,
        /// Export Carapace specification (YAML).
        #[arg(long = "carapace")]
        carapace: bool,
        /// Install completion script automatically to the active shell's profile/config.
        #[arg(long = "install")]
        install: bool,
    },

    /// Dynamic context-aware Carapace bridge providing format recommendations.
    #[command(name = "_carapace", hide = true)]
    Carapace {
        /// Shell identifier.
        shell: Option<String>,
        /// Arguments typed by the user.
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Display Unicode box table help and usage manual.
    HelpManual,

    /// Display MOSSAMBI sweet lime fruit ASCII art and curvy banner.
    Banner,

    /// Manage background resident daemon (Win+Shift+C / Win+Ctrl+C).
    Daemon {
        /// Action: 'run', 'start', 'stop', 'status', 'install', 'uninstall', or 'prompt'.
        #[arg(default_value = "prompt")]
        action: String,
    },

    /// Filesystem operations: copy, move, rename, delete-to-trash.
    Fs {
        /// Filesystem action to perform.
        #[command(subcommand)]
        action: FsCommand,
    },

    /// Manage file tags (list, add, remove, find).
    Tags {
        /// Tag action to perform.
        #[command(subcommand)]
        action: TagsCommand,
    },

    /// Manage the conversion queue (add, list, run).
    Queue {
        /// Queue action to perform.
        #[command(subcommand)]
        action: QueueCommand,
    },

    /// Open With: list registered handlers, launch one, or use custom items.
    OpenWith {
        /// Open With action to perform.
        #[command(subcommand)]
        action: OpenWithCommand,
    },

    /// Print a text preview and metadata for a file.
    Preview {
        /// File to preview.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Crop an image to a normalized region.
    Crop {
        /// Source image file.
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Left edge (0.0 - 1.0).
        #[arg(long = "x", default_value_t = 0.05)]
        x: f32,
        /// Top edge (0.0 - 1.0).
        #[arg(long = "y", default_value_t = 0.05)]
        y: f32,
        /// Crop width (0.0 - 1.0).
        #[arg(long = "w", default_value_t = 0.9)]
        w: f32,
        /// Crop height (0.0 - 1.0).
        #[arg(long = "h", default_value_t = 0.9)]
        h: f32,
        /// Aspect ratio preset: free, square, 4:3, 16:9, 3:2, 5:4.
        #[arg(long = "aspect")]
        aspect: Option<String>,
        /// Output image path (defaults to a sibling *_crop file).
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
    },

    /// List or run user scripts (Rhai / Lua).
    Scripts {
        /// Script action to perform.
        #[command(subcommand)]
        action: ScriptsCommand,
    },

    /// Show the Dijkstra conversion route between two formats.
    Route {
        /// Source format/extension.
        #[arg(value_name = "FROM")]
        from: String,
        /// Target format/extension.
        #[arg(value_name = "TO")]
        to: String,
    },

    /// Fuzzy filename search under a root directory (global search).
    Search {
        /// Search query.
        #[arg(value_name = "QUERY")]
        query: String,
        /// Root directory to search (defaults to current directory).
        #[arg(short = 'r', long = "root")]
        root: Option<PathBuf>,
        /// Maximum number of results.
        #[arg(long = "limit", default_value_t = 50)]
        limit: usize,
    },

    /// Pattern-based batch rename (dry-run preview by default).
    BatchRename {
        /// Files to rename.
        #[arg(required = true, value_name = "FILE")]
        files: Vec<PathBuf>,
        /// Text to find in each file stem.
        #[arg(long = "find")]
        find: String,
        /// Replacement text.
        #[arg(long = "replace", default_value_t = String::new())]
        replace: String,
        /// Optional prefix.
        #[arg(long = "prefix")]
        prefix: Option<String>,
        /// Optional suffix.
        #[arg(long = "suffix")]
        suffix: Option<String>,
        /// Starting number for the %N sequence token.
        #[arg(long = "start", default_value_t = 1)]
        start: usize,
        /// Actually apply the renames (default is a dry-run preview).
        #[arg(long = "apply")]
        apply: bool,
    },

    /// Undo the last journaled action; redo is N/A (no core redo API).
    Undo,

    /// Get, set, or list every configuration field generically.
    Settings {
        /// Settings action to perform.
        #[command(subcommand)]
        action: SettingsCommand,
    },
}

/// Filesystem mutation command group.
#[derive(Subcommand, Debug, PartialEq)]
pub enum FsCommand {
    /// Copy one or more sources into a destination directory or file.
    Copy {
        /// Source paths to copy (one or more).
        #[arg(required = true, value_name = "SOURCE")]
        sources: Vec<PathBuf>,
        /// Destination directory (or file path for a single source).
        #[arg(value_name = "DEST")]
        destination: PathBuf,
    },
    /// Move one or more sources into a destination directory or file.
    Move {
        /// Source paths to move (one or more).
        #[arg(required = true, value_name = "SOURCE")]
        sources: Vec<PathBuf>,
        /// Destination directory (or file path for a single source).
        #[arg(value_name = "DEST")]
        destination: PathBuf,
    },
    /// Rename a file or directory.
    Rename {
        /// Path to the item to rename.
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// New name for the item (a single path component).
        #[arg(value_name = "NEW_NAME")]
        new_name: String,
    },
    /// Delete items to the OS Recycle Bin / Trash (safe by default).
    Delete {
        /// Paths to delete to trash.
        #[arg(required = true, value_name = "PATH")]
        paths: Vec<PathBuf>,
        /// Permanently delete instead of moving to trash.
        #[arg(long = "permanent")]
        permanent: bool,
    },
}

/// Tag management command group.
#[derive(Subcommand, Debug, PartialEq)]
pub enum TagsCommand {
    /// List tags on a path, or all available tags when no path is given.
    List {
        /// Optional path whose tags should be listed.
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
    },
    /// Add one or more tags to a path.
    Add {
        /// Path to tag.
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Tags to add.
        #[arg(required = true, value_name = "TAG")]
        tags: Vec<String>,
    },
    /// Remove one or more tags from a path.
    Remove {
        /// Path to untag.
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Tags to remove.
        #[arg(required = true, value_name = "TAG")]
        tags: Vec<String>,
    },
    /// Find files carrying a tag.
    Find {
        /// Tag to search for.
        #[arg(value_name = "TAG")]
        tag: String,
    },
}

/// Conversion queue command group.
#[derive(Subcommand, Debug, PartialEq)]
pub enum QueueCommand {
    /// Add files to the persistent conversion queue.
    Add {
        /// Source files to enqueue.
        #[arg(required = true, value_name = "FILE")]
        files: Vec<PathBuf>,
        /// Target format for every queued file.
        #[arg(short = 't', long = "target", value_name = "FMT")]
        target: String,
    },
    /// List queued conversion jobs.
    List,
    /// Run all pending queued conversion jobs.
    Run,
}

/// Open With command group.
#[derive(Subcommand, Debug, PartialEq)]
pub enum OpenWithCommand {
    /// List registered applications for a file plus custom menu items.
    List {
        /// File whose handlers should be listed.
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },
    /// Launch a registered handler by its 1-based index from `open-with list`.
    Run {
        /// File to open.
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// 1-based handler index.
        #[arg(value_name = "INDEX")]
        index: usize,
    },
    /// List custom context-menu items from configuration.
    CustomList,
    /// Run a configured custom context-menu item.
    CustomRun {
        /// Label of the custom item (substring match, case-insensitive).
        #[arg(value_name = "LABEL")]
        label: String,
        /// Optional file path passed to the item as %p.
        #[arg(value_name = "FILE")]
        file: Option<PathBuf>,
    },
}

/// Scripts command group.
#[derive(Subcommand, Debug, PartialEq)]
pub enum ScriptsCommand {
    /// List user scripts discovered on disk.
    List,
    /// Run a script by its id (substring match, case-insensitive).
    Run {
        /// Script id or title fragment.
        #[arg(value_name = "ID")]
        id: String,
        /// Optional current file passed to the script.
        #[arg(long = "file")]
        file: Option<PathBuf>,
    },
}

/// Settings command group (generic over every config field).
#[derive(Subcommand, Debug, PartialEq)]
pub enum SettingsCommand {
    /// List every configuration field as dotted key = value lines.
    List {
        /// Optional key prefix to restrict the listing.
        #[arg(value_name = "PREFIX")]
        prefix: Option<String>,
    },
    /// Get the value of a dotted configuration key.
    Get {
        /// Dotted key path (e.g. appearance.glass or donut.appearance.size).
        #[arg(value_name = "KEY")]
        key: String,
    },
    /// Set a dotted configuration key to a JSON-typed value.
    Set {
        /// Dotted key path.
        #[arg(value_name = "KEY")]
        key: String,
        /// New value (JSON literal when possible, otherwise a string).
        #[arg(value_name = "VALUE")]
        value: String,
    },
}

/// Run the CLI. Returns the numeric process exit code.
pub fn run() -> i32 {
    let _ = env_logger::try_init();
    let args: Vec<String> = std::env::args().collect();

    // Platform launch intents (Explorer shell forms, deep links, no-arg
    // activation) use argument shapes clap cannot model. Detect and run those
    // first, then hand everything else to clap.
    if let Some(code) = launch::handle_platform_launch(&args) {
        return code;
    }

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            use clap::error::ErrorKind;
            let code = match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => ExitCode::Usage.code(),
            };
            let _ = e.print();
            return code;
        }
    };

    result_code(dispatch(cli))
}

/// Convert a command result into a process exit code.
pub fn result_code(result: Result<()>) -> i32 {
    match result {
        Ok(()) => ExitCode::Ok.code(),
        Err(err) => {
            eprintln!("error: {err:#}");
            output::exit_code_for_error(&err).code()
        }
    }
}

/// Dispatch a parsed CLI to its command group handler.
pub fn dispatch(cli: Cli) -> Result<()> {
    let json = cli.json;
    let strict = cli.strict_lossless;

    // Flag-style operations.
    if cli.banner {
        help::print_welcome_banner();
        return Ok(());
    }
    if cli.manual {
        help::print_table_help();
        return Ok(());
    }
    if cli.hw {
        return inspect::handle_hw(json);
    }
    if cli.themes {
        return inspect::handle_themes(json);
    }
    if let Some(ref check_file) = cli.check {
        return inspect::handle_check(check_file, json);
    }
    if let Some(ref bench_file) = cli.bench {
        return inspect::handle_bench(bench_file, cli.target.as_deref(), json);
    }
    if let Some(ref verify_files) = cli.verify {
        if verify_files.len() >= 2 {
            return inspect::handle_verify(&verify_files[0], &verify_files[1], json);
        }
        anyhow::bail!("--verify requires two file arguments: <original> <converted>");
    }
    if let Some(ref query) = cli.catalogue {
        return inspect::handle_catalogue(query, json);
    }
    if let Some(ref inputs) = cli.archive {
        if let Some(ref out) = cli.output_dir {
            return archive::handle_archive(inputs, out, None, json);
        }
        anyhow::bail!("--archive requires -o <OUTPUT_ARCHIVE>");
    }
    if let Some(ref arc) = cli.extract {
        return archive::handle_extract(arc, cli.output_dir.as_deref(), json);
    }
    if let Some(ref d_act) = cli.daemon {
        return crate::daemon::handle_daemon_command(d_act);
    }

    if cli.app {
        let dir = cli.input.clone().filter(|p| p.is_dir());
        return launch::handle_gui(dir);
    }
    if cli.widget {
        return launch::handle_widget(None, false, None);
    }
    if cli.tui {
        return launch::handle_tui(None);
    }

    // Explicit subcommands.
    if let Some(cmd) = cli.command {
        return match cmd {
            Commands::App { path } | Commands::Explorer { path } => launch::handle_gui(path),
            Commands::Tui { path } => launch::handle_tui(path),
            Commands::Widget {
                pos,
                active_drag,
                file,
            } => launch::handle_widget(pos.as_deref(), active_drag, file),
            Commands::Convert {
                input,
                target,
                output,
                all,
            } => {
                convert::handle_convert(&input, Some(&target), all, output.as_deref(), strict, json)
            }
            Commands::Archive {
                inputs,
                output,
                archive_type,
            } => archive::handle_archive(&inputs, &output, archive_type.as_deref(), json),
            Commands::Extract {
                archive,
                output_dir,
            } => archive::handle_extract(&archive, output_dir.as_deref(), json),
            Commands::Info { file } => inspect::handle_info(&file, json),
            Commands::Check { file } => inspect::handle_check(&file, json),
            Commands::Bench { file, target } => {
                inspect::handle_bench(&file, target.as_deref(), json)
            }
            Commands::Hw => inspect::handle_hw(json),
            Commands::Verify { source, target } => inspect::handle_verify(&source, &target, json),
            Commands::Catalogue { query } => inspect::handle_catalogue(&query, json),
            Commands::UploadReady {
                input,
                preset,
                width,
                height,
                dpi,
                min_kb,
                max_kb,
                whiten,
                output_dir,
            } => convert::handle_exam(
                &input,
                preset.as_deref(),
                width,
                height,
                dpi,
                min_kb,
                max_kb,
                whiten,
                output_dir.as_deref(),
                json,
            ),
            Commands::Themes => inspect::handle_themes(json),
            Commands::Config { action, key, value } => {
                config_cli::handle_config(&action, key.as_deref(), value.as_deref(), json)
            }
            Commands::Completions {
                shell,
                carapace,
                install,
            } => config_cli::handle_completions(shell.as_deref(), carapace, install),
            Commands::Carapace { shell, args } => {
                config_cli::handle_carapace(shell.as_deref(), &args)
            }
            Commands::HelpManual => {
                help::print_table_help();
                Ok(())
            }
            Commands::Banner => {
                help::print_welcome_banner();
                Ok(())
            }
            Commands::Daemon { action } => crate::daemon::handle_daemon_command(&action),
            Commands::Fs { action } => fs_ops::handle_fs(action, json),
            Commands::Tags { action } => tags_cli::handle_tags(action, json),
            Commands::Queue { action } => queue_cli::handle_queue(action, json),
            Commands::OpenWith { action } => open_with::handle_open_with(action, json),
            Commands::Preview { file } => preview::handle_preview(&file, json),
            Commands::Crop {
                file,
                x,
                y,
                w,
                h,
                aspect,
                output,
            } => crop::handle_crop(
                &file,
                x,
                y,
                w,
                h,
                aspect.as_deref(),
                output.as_deref(),
                json,
            ),
            Commands::Scripts { action } => scripts::handle_scripts(action, json),
            Commands::Route { from, to } => route::handle_route(&from, &to, json),
            Commands::Search { query, root, limit } => {
                search::handle_search(&query, root.as_deref(), limit, json)
            }
            Commands::BatchRename {
                files,
                find,
                replace,
                prefix,
                suffix,
                start,
                apply,
            } => batch_rename::handle_batch_rename(
                &files,
                &find,
                &replace,
                prefix.as_deref(),
                suffix.as_deref(),
                start,
                apply,
                json,
            ),
            Commands::Undo => undo::handle_undo(json),
            Commands::Settings { action } => settings::handle_settings(action, json),
        };
    }

    // UNIX mv-like positional syntax: mossambi <INPUT> [TARGET_FORMAT].
    if let Some(input) = cli.input {
        let input_str = input.to_string_lossy();
        match input_str.as_ref() {
            "app" | "explorer" | "gui" => return launch::handle_gui(None),
            "widget" => return launch::handle_widget(None, false, None),
            "tui" => return launch::handle_tui(None),
            "banner" => {
                help::print_welcome_banner();
                return Ok(());
            }
            "help-manual" | "manual" | "help" => {
                help::print_table_help();
                return Ok(());
            }
            "hw" => return inspect::handle_hw(json),
            "themes" => return inspect::handle_themes(json),
            _ => {}
        }

        if input.is_dir() && cli.target.is_none() && !cli.all && cli.exam_preset.is_none() {
            return launch::handle_gui(Some(input));
        }

        if cli.exam_preset.is_some() || cli.exam_width.is_some() || cli.exam_min_kb.is_some() {
            return convert::handle_exam(
                &input,
                cli.exam_preset.as_deref(),
                cli.exam_width,
                cli.exam_height,
                cli.exam_dpi,
                cli.exam_min_kb,
                cli.exam_max_kb,
                cli.exam_whiten,
                cli.output_dir.as_deref(),
                json,
            );
        }

        if cli.all || cli.target.as_deref().map(|t| t.to_uppercase()) == Some("ALL".to_string()) {
            return convert::handle_convert(
                &input,
                Some("ALL"),
                true,
                cli.output_dir.as_deref(),
                strict,
                json,
            );
        }

        if let Some(ref target) = cli.target {
            return convert::handle_convert(
                &input,
                Some(target),
                false,
                cli.output_dir.as_deref(),
                strict,
                json,
            );
        }

        return inspect::handle_info(&input, json);
    }

    launch::handle_gui(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        let mut full = vec!["mossambi"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("arguments should parse")
    }

    #[test]
    fn no_args_launches_gui_by_default() {
        let cli = parse(&[]);
        assert!(cli.input.is_none());
        assert!(cli.command.is_none());
        assert!(!cli.tui);
    }

    #[test]
    fn tui_flag_parses() {
        let cli = parse(&["--tui"]);
        assert!(cli.tui);
    }

    #[test]
    fn opening_a_path_parses_as_input() {
        let cli = parse(&["/some/path"]);
        assert_eq!(cli.input, Some(PathBuf::from("/some/path")));
    }

    #[test]
    fn mv_style_conversion_parses() {
        let cli = parse(&["in.wav", "flac"]);
        assert_eq!(cli.input, Some(PathBuf::from("in.wav")));
        assert_eq!(cli.target.as_deref(), Some("flac"));
    }

    #[test]
    fn json_is_global() {
        let cli = parse(&["--json", "check", "file.txt"]);
        assert!(cli.json);
        assert!(matches!(cli.command, Some(Commands::Check { .. })));
        // global flag must also parse after the subcommand
        let cli2 = parse(&["config", "list", "--json"]);
        assert!(cli2.json);
    }

    #[test]
    fn completions_subcommand_parses() {
        let cli = parse(&["completions", "powershell"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Completions {
                ref shell,
                carapace: false,
                install: false,
            }) if shell.as_deref() == Some("powershell")
        ));

        let cli_install = parse(&["completions", "--install"]);
        assert!(matches!(
            cli_install.command,
            Some(Commands::Completions {
                shell: None,
                carapace: false,
                install: true,
            })
        ));
    }

    #[test]
    fn fs_subcommands_parse() {
        let cli = parse(&["fs", "copy", "a.txt", "b.txt", "dest"]);
        assert!(matches!(cli.command, Some(Commands::Fs { .. })));
        let cli = parse(&["fs", "rename", "a.txt", "b.txt"]);
        assert!(matches!(cli.command, Some(Commands::Fs { .. })));
    }

    #[test]
    fn tags_subcommands_parse() {
        let cli = parse(&["tags", "add", "file.txt", "Home", "Work"]);
        assert!(matches!(cli.command, Some(Commands::Tags { .. })));
        let cli = parse(&["tags", "find", "Important"]);
        assert!(matches!(cli.command, Some(Commands::Tags { .. })));
    }

    #[test]
    fn queue_subcommands_parse() {
        let cli = parse(&["queue", "add", "a.txt", "b.txt", "--target", "pdf"]);
        assert!(matches!(cli.command, Some(Commands::Queue { .. })));
        let cli = parse(&["queue", "run"]);
        assert!(matches!(cli.command, Some(Commands::Queue { .. })));
    }

    #[test]
    fn help_text_present_on_every_command_group() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        for name in [
            "fs",
            "tags",
            "queue",
            "completions",
            "convert",
            "archive",
            "open-with",
            "preview",
            "crop",
            "scripts",
            "route",
            "search",
            "batch-rename",
            "undo",
            "settings",
        ] {
            let sub = cmd.get_subcommands().find(|c| c.get_name() == name);
            assert!(sub.is_some(), "missing subcommand {name}");
            let about = sub
                .unwrap()
                .get_about()
                .or_else(|| sub.unwrap().get_long_about());
            assert!(about.is_some(), "subcommand {name} has no help text");
        }
    }

    #[test]
    fn open_with_subcommands_parse() {
        let cli = parse(&["open-with", "list", "file.txt"]);
        assert!(matches!(cli.command, Some(Commands::OpenWith { .. })));
        let cli = parse(&["open-with", "run", "file.txt", "1"]);
        assert!(matches!(cli.command, Some(Commands::OpenWith { .. })));
        let cli = parse(&["open-with", "custom-list"]);
        assert!(matches!(cli.command, Some(Commands::OpenWith { .. })));
    }

    #[test]
    fn preview_and_crop_parse() {
        let cli = parse(&["preview", "file.txt"]);
        assert!(matches!(cli.command, Some(Commands::Preview { .. })));
        let cli = parse(&["crop", "img.png", "--x", "0.1", "--aspect", "16:9"]);
        assert!(matches!(cli.command, Some(Commands::Crop { .. })));
    }

    #[test]
    fn scripts_route_search_parse() {
        let cli = parse(&["scripts", "list"]);
        assert!(matches!(cli.command, Some(Commands::Scripts { .. })));
        let cli = parse(&["scripts", "run", "abc", "--file", "in.txt"]);
        assert!(matches!(cli.command, Some(Commands::Scripts { .. })));
        let cli = parse(&["route", "hwp", "pdf"]);
        assert!(matches!(cli.command, Some(Commands::Route { .. })));
        let cli = parse(&["search", "needle", "--root", ".", "--limit", "5"]);
        assert!(matches!(cli.command, Some(Commands::Search { .. })));
    }

    #[test]
    fn batch_rename_undo_settings_parse() {
        let cli = parse(&[
            "batch-rename",
            "a.txt",
            "b.txt",
            "--find",
            "a",
            "--replace",
            "z",
        ]);
        assert!(matches!(cli.command, Some(Commands::BatchRename { .. })));
        let cli = parse(&["undo"]);
        assert!(matches!(cli.command, Some(Commands::Undo)));
        let cli = parse(&["settings", "get", "theme_mode"]);
        assert!(matches!(cli.command, Some(Commands::Settings { .. })));
        let cli = parse(&["settings", "set", "always_on_top", "true"]);
        assert!(matches!(cli.command, Some(Commands::Settings { .. })));
    }

    #[test]
    fn batch_rename_is_dry_run_by_default() {
        let cli = parse(&["batch-rename", "a.txt", "--find", "a"]);
        match cli.command {
            Some(Commands::BatchRename { apply, .. }) => assert!(!apply),
            _ => panic!("expected batch-rename"),
        }
        let cli = parse(&["batch-rename", "a.txt", "--find", "a", "--apply"]);
        match cli.command {
            Some(Commands::BatchRename { apply, .. }) => assert!(apply),
            _ => panic!("expected batch-rename"),
        }
    }
}
