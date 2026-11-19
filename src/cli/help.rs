//! Banner and manual help output. Pure ASCII by contract.

/// Print the MOSSAMBI banner and access-modality summary.
pub fn print_welcome_banner() {
    let sage = "\x1b[38;2;74;133;90m";
    let blue = "\x1b[38;2;0;122;255m";
    let amber = "\x1b[38;2;217;119;6m";
    let cyan = "\x1b[38;2;60;200;230m";
    let white = "\x1b[38;2;244;245;247m";
    let muted = "\x1b[38;2;160;160;165m";
    let bold = "\x1b[1m";
    let reset = "\x1b[0m";

    println!(
        r#"{sage}
                       .-''''-.     _/\_
                     .'  ____  `.  (____)
                    /  .' || `.  \   ||
                   |  / \ || / \  |  ||
                   | ==  (())  == |  ||
                   |  \ / || \ /  |  ||
                    \  `. || .'  /   ||
                     `.  `--'  .'
                       `'-..-'`{reset}

{blue}   M O S S A M B I{reset}
{amber}   Zero-Loss File Conversion Platform & Universal File Explorer{reset}
{muted}   One Pure Rust Core - Three Modalities: Explorer App, Radial Widget, Interactive TUI{reset}
   {white}v2.1.0 | 0.00% Zero-Loss Standard | Windows Fluent & macOS Finder DNA{reset}
 "#
    );

    println!("{bold}Access Modalities:{reset}");
    println!("  {cyan}mossambi{reset}                       # Launch full macOS-style File Explorer & Studio GUI");
    println!("  {cyan}mossambi app [DIR]{reset}             # Launch Explorer GUI opened at specified directory");
    println!(
        "  {cyan}mossambi explorer [DIR]{reset}        # Launch Explorer GUI (explicit alias)"
    );
    println!("  {cyan}mossambi widget{reset}                # Summon Sweet Lime desktop floating radial widget");
    println!("  {cyan}mossambi tui [DIR]{reset}             # Launch Yazi-grade interactive Terminal File Explorer");
    println!();
    println!("{bold}CLI Fast-Path Conversion:{reset}");
    println!("  {cyan}mossambi <file> [format]{reset}       # Instant conversion (e.g. mossambi sound.wav flac)");
    println!("  {cyan}mossambi <file> --all{reset}          # Transcode to ALL domain formats in parallel");
    println!("  {cyan}mossambi <photo> --exam <p>{reset}    # Indian Exam Optimizer (upsc-photo, ssc-photo, etc.)");
    println!("  {cyan}mossambi archive <f...> -o a.zip{reset}# Exhaustive compression archive (ZIP, TAR, ZST)");
    println!(
        "  {cyan}mossambi extract <archive>{reset}     # Safe extraction with Zip-Slip protection"
    );
    println!(
        "  {cyan}mossambi check <file>{reset}          # Quality, integrity & decode scorecard"
    );
    println!("  {cyan}mossambi bench <file> [tgt]{reset}    # Latency, throughput (MB/s) & speedup benchmark");
    println!(
        "  {cyan}mossambi hw{reset}                    # GPU/NPU & SIMD hardware acceleration info"
    );
    println!("  {cyan}mossambi verify <src> <tgt>{reset}    # Mathematical 0.00% zero-loss verification proof");
    println!(
        "  {cyan}mossambi catalogue [query]{reset}     # Search 200+ format registry catalogue"
    );
    println!("  {cyan}mossambi themes{reset}                # List named eye-safe themes");
    println!("  {cyan}mossambi fs <copy|move|rename|delete>{reset} # Filesystem operations");
    println!("  {cyan}mossambi tags <list|add|remove|find>{reset}  # File tagging engine");
    println!("  {cyan}mossambi queue <add|list|run>{reset}   # Conversion queue control");
    println!("  {cyan}mossambi daemon{reset}                # Resident mouse gesture service (Win+Shift+C / Win+Ctrl+C)");
    println!("  {cyan}mossambi help-manual{reset}           # Exhaustive table help and manual");
    println!("\nRun '{cyan}mossambi --help{reset}' for all command flags and options.\n");
}

/// Print the ASCII box table manual (the `help-manual` surface).
pub fn print_table_help() {
    println!("+------------------------------------------------------------------------------------------------+");
    println!("|  Mossambi (MOSSAMBI - ZERO-LOSS CONVERSION PLATFORM & UNIVERSAL FILE EXPLORER v2.1.0)            |");
    println!("|  One Pure Rust Core - Three Operating Modalities - Apple Elegance + Linux Power                  |");
    println!("+------------------------------------------------------------------------------------------------+");
    println!("|  Usage: mossambi [FLAGS / OPTIONS] [INPUT] [TARGET_FORMAT]                                       |");
    println!("|  UNIX 'mv' POSITIONAL GRAMMAR: mossambi <INPUT> [TARGET_FORMAT] [-a|--all] [-o <DIR>]            |");
    println!("+--------------------------+----------------------------------------------+-----------------------+");
    println!("|  FLAG / OPTION           |  DESCRIPTION & FUNCTION                       |  EXAMPLE USAGE        |");
    println!("+--------------------------+----------------------------------------------+-----------------------+");
    println!("|  --app, --explorer [DIR] |  Launch macOS-styled Universal File Explorer |  mossambi --app       |");
    println!("|  --widget                |  Launch Sweet Lime Radial Widget             |  mossambi --widget    |");
    println!("|  --tui [DIR]             |  Launch interactive Terminal File Explorer   |  mossambi --tui       |");
    println!("|  -a, --all               |  Batch transcode to ALL valid formats        |  mossambi f.wav -a    |");
    println!("|  -o, --output <DIR>      |  Custom destination directory for output     |  mossambi f.png -o .  |");
    println!("|  --strict                |  Enforce strict 0.00% zero-loss standard     |  mossambi in.wav flac |");
    println!("|  --json                  |  Output structured machine-parseable JSON    |  mossambi in.wav --json|");
    println!("|  --exam <PRESET>         |  Indian Exam Tuner (upsc-photo, ssc-photo)    |  mossambi in.jpg --exam|");
    println!("|  --width / --height      |  Exact target width and height in pixels     |  --width 350          |");
    println!("|  --dpi <INT>             |  Target resolution DPI (e.g. 200 or 300)     |  --dpi 300            |");
    println!("|  --min-kb / --max-kb     |  Target file byte envelope range             |  --min-kb 20          |");
    println!("|  --whiten                |  Auto-whiten scanned signature backgrounds   |  --whiten             |");
    println!("|  --archive <FILES...>    |  Pack into ZIP, TAR, GZ, BZ2, XZ, or ZST      |  --archive a b -o c.zip|");
    println!("|  --extract <ARCHIVE>     |  Extract with Zip-Slip protection            |  --extract b.zip      |");
    println!("|  --check <FILE>          |  Quality, integrity & decode scorecard       |  --check f.png        |");
    println!("|  --bench <FILE>          |  Latency & throughput benchmark             |  --bench f.png        |");
    println!("|  --verify <SRC> <TGT>    |  Mathematical zero-loss verification         |  --verify a b         |");
    println!("|  --catalogue [QUERY]     |  Search 200+ format registry catalogue       |  --catalogue pdf      |");
    println!("|  --hw                    |  GPU/NPU & SIMD hardware diagnostics         |  mossambi --hw        |");
    println!("|  --themes                |  List all named Pantone themes              |  mossambi --themes    |");
    println!("|  --daemon <ACTION>       |  Manage background resident daemon          |  --daemon status      |");
    println!("|  fs <ACTION>             |  copy / move / rename / delete-to-trash     |  mossambi fs copy ... |");
    println!("|  tags <ACTION>           |  list / add / remove / find                 |  mossambi tags find X |");
    println!("|  queue <ACTION>          |  add / list / run conversion queue          |  mossambi queue run   |");
    println!("|  completions <SHELL>     |  Generate bash/zsh/fish/powershell/elvish   |  completions bash     |");
    println!("|  config <get|set|list>   |  View or edit persistent configuration       |  config get theme     |");
    println!("+--------------------------+----------------------------------------------+-----------------------+");
    println!("\nRun 'mossambi --help' for clap-generated help covering every command and flag.\n");
}
