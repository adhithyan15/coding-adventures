use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_file, Parser};
// `layout_ir` is a pure-data crate that builds on every target, so the pieces
// used to *describe* the PNG scene stay un-gated on purpose — see the note on
// `cowsay_text_content` below for why that matters. Only the imports that pull
// in Apple-only machinery (Metal, CoreText) are gated.
use layout_ir::{color_black, font_spec, FontSpec, TextAlign, TextContent};
// These layout/text imports are only used by the Apple-only PNG rendering path
// below (`#[cfg(target_vendor = "apple")]`). Gate them the same way so non-Apple
// targets (e.g. Linux CI) don't flag them as unused.
#[cfg(target_vendor = "apple")]
use layout_ir::{color_white, Content, PositionedNode, TextMeasurer};
#[cfg(target_vendor = "apple")]
use layout_text_measure_native::NativeMeasurer;
#[cfg(target_vendor = "apple")]
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use regex::Regex;
use serde_json::Value;
#[cfg(target_vendor = "apple")]
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(target_vendor = "apple")]
use text_native::text_interfaces::{FontMetrics, FontResolver, TextShaper};
#[cfg(target_vendor = "apple")]
use text_native::{NativeMetrics, NativeResolver, NativeShaper};

#[derive(Debug, Default)]
struct PaintRenderOptions {
    png_output: Option<PathBuf>,
    random_text: bool,
}

const RANDOM_MESSAGES: &[&str] = &[
    "Paint VM says hello from native glyph runs",
    "Moo, but make it Metal",
    "ASCII art survives the layout pipeline",
    "This cow was shaped by CoreText",
    "The bubble is text, the pixels are native",
];

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if text.len() <= width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec!["".to_string()];
    }

    let mut current_line = String::new();
    for word in words {
        if current_line.len() + word.len() < width {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else {
                current_line.push(' ');
                current_line.push_str(word);
            }
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

fn format_bubble(lines: &[String], is_think: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let max_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let border_top = format!(" {}", "_".repeat(max_len + 2));
    let border_bottom = format!(" {}", "-".repeat(max_len + 2));

    let mut result = vec![border_top];

    if lines.len() == 1 {
        let (start, end) = if is_think { ("(", ")") } else { ("<", ">") };
        result.push(format!(
            "{} {:<width$} {}",
            start,
            lines[0],
            end,
            width = max_len
        ));
    } else {
        for (i, line) in lines.iter().enumerate() {
            let (start, end) = if is_think {
                ("(", ")")
            } else if i == 0 {
                ("/", "\\")
            } else if i == lines.len() - 1 {
                ("\\", "/")
            } else {
                ("|", "|")
            };
            result.push(format!(
                "{} {:<width$} {}",
                start,
                line,
                end,
                width = max_len
            ));
        }
    }

    result.push(border_bottom);
    result.join("\n")
}

// ═══════════════════════════════════════════════════════════════════════════
// Choosing a cow file safely
// ═══════════════════════════════════════════════════════════════════════════
//
// `-f NAME` / `--file NAME` picks which cow to draw. The name is glued into a
// path — `<cows dir>/NAME.cow` — so it is untrusted input that decides which
// file this program opens and echoes back to stdout. Left unchecked, a name
// can walk out of the cows directory (issue #12169):
//
//   name given                    path actually opened
//   ───────────────────────────── ─────────────────────────────────────────
//   tux                           code/specs/cows/tux.cow          (fine)
//   ../../../../home/me/notes     /home/me/notes.cow               (escape)
//   /etc/secret                   /etc/secret.cow                  (escape —
//                                 joining an absolute path REPLACES the base)
//   C:\Users\me\x                 C:\Users\me\x.cow                (Windows)
//
// The forced `.cow` suffix limits what can be read, and a local CLI already
// runs with its caller's privileges — but a wrapper script, web service or CI
// job that forwards someone else's string to `cowsay -f` would hand that
// someone a file-read primitive. So we defend in two layers, mirroring the
// C#, F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:
//
//   1. Syntactic check (`is_safe_cow_name`). A cow name is a bare file stem,
//      so anything that could mean "some other directory" is refused:
//        `/` `\`   path separators (`\` is one on Windows)
//        `..`      parent-directory steps
//        `:`       Windows drive (`C:x`) and alternate-data-stream syntax
//        NUL       C APIs stop reading at it, so the OS would see a
//                  different name from the one we checked
//        ""        the empty name
//      An absolute path always contains `/`, `\` or `:`, so it is refused by
//      the same rule.
//   2. Containment check. After joining, both the cows directory and the
//      candidate are canonicalized (symlinks and `.`/`..` resolved by the OS)
//      and the candidate must still live inside the cows directory. This
//      catches what string inspection cannot see — e.g. a symlink planted in
//      the cows directory that points somewhere else.
//
// A name that fails either layer is treated exactly like a cow that does not
// exist: we quietly draw `default.cow`. That matches every other port and
// keeps cowsay's long-standing "unknown cow → default cow" behaviour.
//
// Nothing is URL-decoded: `%2F` is three literal characters to the operating
// system, never a slash, so `..%2Fx` is just an odd (and here, rejected) name.

/// True when `cow_name` is a bare file stem that cannot name another directory.
fn is_safe_cow_name(cow_name: &str) -> bool {
    !cow_name.is_empty()
        && !cow_name.contains("..")
        && !cow_name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '\0'))
}

/// Resolve `cow_name` to a readable `.cow` file inside `cows_dir`, falling back
/// to `cows_dir/default.cow` when the name is unsafe, missing, or escapes.
fn resolve_cow_path(cow_name: &str, cows_dir: &Path) -> PathBuf {
    let default_path = cows_dir.join("default.cow");
    if !is_safe_cow_name(cow_name) {
        return default_path;
    }

    let candidate = cows_dir.join(format!("{}.cow", cow_name));
    // `canonicalize` fails for a file that does not exist, which is exactly the
    // "unknown cow" case — so one match covers both "missing" and "escaped".
    match (fs::canonicalize(cows_dir), fs::canonicalize(&candidate)) {
        (Ok(root), Ok(real)) if real.starts_with(&root) && real.is_file() => real,
        _ => default_path,
    }
}

fn load_cow(cow_name: &str, cows_dir: &Path) -> String {
    let cow_path = resolve_cow_path(cow_name, cows_dir);

    let content = fs::read_to_string(cow_path).unwrap_or_else(|_| "Error loading cow".to_string());

    let re = Regex::new(r"(?s)<<EOC;\n(.*?)EOC").unwrap();
    if let Some(caps) = re.captures(&content) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }
    content
}

fn find_root() -> PathBuf {
    let mut curr = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..10 {
        if curr.join("code/specs/cowsay.json").exists() {
            return curr;
        }
        if let Some(parent) = curr.parent() {
            curr = parent.to_path_buf();
        } else {
            break;
        }
    }
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let root = find_root();
    let spec_path = root.join("code/specs/cowsay.json");
    let spec = load_spec_from_file(spec_path.to_str().unwrap()).expect("Failed to load spec");

    let parser = Parser::new(spec);
    let (args, render_options) = extract_paint_render_options(env::args().collect());

    match parser.parse(&args) {
        Ok(ParserOutput::Parse(result)) => handle_parse_result(result, &root, &render_options),
        Ok(ParserOutput::Help(h)) => {
            print!("{}", h.text);
            if render_options.png_output.is_some() || render_options.random_text {
                println!("\nPaint VM options:");
                println!("      --png <PATH>       Render Cowsay as a PNG via paint-metal");
                println!("      --png-metal <PATH> Alias for --png");
                println!("      --random-text      Use a built-in random message");
            }
        }
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn extract_paint_render_options(args: Vec<String>) -> (Vec<String>, PaintRenderOptions) {
    let mut filtered = Vec::with_capacity(args.len());
    let mut options = PaintRenderOptions::default();
    let mut iter = args.into_iter();
    let mut saw_message = false;

    if let Some(program) = iter.next() {
        filtered.push(program);
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-e" | "--eyes" | "-T" | "--tongue" | "-f" | "--file" | "-W" => {
                filtered.push(arg.clone());
                let Some(value) = iter.next() else {
                    eprintln!("cowsay: {} requires a value", arg);
                    std::process::exit(2);
                };
                filtered.push(value);
            }
            "--png" | "--png-metal" => {
                let Some(path) = iter.next() else {
                    eprintln!("cowsay: {} requires a path argument", arg);
                    std::process::exit(2);
                };
                options.png_output = Some(PathBuf::from(path));
            }
            "--random-text" => {
                options.random_text = true;
            }
            _ if arg.starts_with("--png=") => {
                options.png_output = Some(PathBuf::from(arg.trim_start_matches("--png=")));
            }
            _ if arg.starts_with("--png-metal=") => {
                options.png_output = Some(PathBuf::from(arg.trim_start_matches("--png-metal=")));
            }
            _ => {
                if !arg.starts_with('-') {
                    saw_message = true;
                }
                filtered.push(arg);
            }
        }
    }

    if options.random_text && !saw_message {
        filtered.push(random_message());
    }

    (filtered, options)
}

fn handle_parse_result(
    result: cli_builder::types::ParseResult,
    root: &Path,
    render_options: &PaintRenderOptions,
) {
    let Some(output) = build_cowsay_output(result, root, render_options) else {
        return;
    };

    if let Some(path) = &render_options.png_output {
        if let Err(e) = render_cowsay_png_metal(&output, path) {
            eprintln!("cowsay: failed to render PNG: {}", e);
            std::process::exit(1);
        }
        eprintln!("cowsay: wrote {}", path.display());
        return;
    }

    println!("{}", output);
}

fn build_cowsay_output(
    result: cli_builder::types::ParseResult,
    root: &Path,
    render_options: &PaintRenderOptions,
) -> Option<String> {
    let flags = result.flags;
    let args = result.arguments;

    // Handle message
    let mut message = String::new();
    if let Some(Value::Array(parts)) = args.get("message") {
        if parts.is_empty() {
            // Read from stdin if it's not a TTY
            if !atty::is(atty::Stream::Stdin) {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer).ok();
                message = buffer.trim().to_string();
            } else if render_options.random_text {
                message = random_message();
            } else {
                return None;
            }
        } else {
            message = parts
                .iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
                .join(" ");
        }
    }

    if message.is_empty() {
        if render_options.random_text {
            message = random_message();
        } else {
            return None;
        }
    }

    // Handle modes
    let mut eyes = flags
        .get("eyes")
        .and_then(|v| v.as_str())
        .unwrap_or("oo")
        .to_string();
    let mut tongue = flags
        .get("tongue")
        .and_then(|v| v.as_str())
        .unwrap_or("  ")
        .to_string();

    if flags.get("borg").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "==".to_string();
    }
    if flags.get("dead").and_then(|v| v.as_bool()).unwrap_or(false) {
        eyes = "XX".to_string();
        tongue = "U ".to_string();
    }
    if flags
        .get("greedy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "$$".to_string();
    }
    if flags
        .get("paranoid")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "@@".to_string();
    }
    if flags
        .get("stoned")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "xx".to_string();
        tongue = "U ".to_string();
    }
    if flags
        .get("tired")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "--".to_string();
    }
    if flags
        .get("wired")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "OO".to_string();
    }
    if flags
        .get("youthful")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        eyes = "..".to_string();
    }

    // Force 2 chars
    eyes = format!("{:<2}", eyes).chars().take(2).collect();
    tongue = format!("{:<2}", tongue).chars().take(2).collect();

    // Handle wrapping
    let mut lines = Vec::new();
    let nowrap = flags
        .get("nowrap")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if nowrap {
        lines = message.split('\n').map(|s| s.to_string()).collect();
    } else {
        let width = flags
            .get("width")
            .and_then(|v| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)))
            .unwrap_or(40) as usize;

        for line in message.split('\n') {
            if line.is_empty() {
                lines.push("".to_string());
            } else {
                lines.extend(wrap_text(line, width));
            }
        }
    }

    // Handle speech vs thought
    let mut is_think = flags
        .get("think")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if let Some(exe) = env::args().next() {
        if exe.ends_with("cowthink") {
            is_think = true;
        }
    }
    let thoughts = if is_think { "o" } else { "\\" };

    // Generate bubble
    let bubble = format_bubble(&lines, is_think);

    // Load and render cow
    let cowfile = flags
        .get("cowfile")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let mut cow_template = load_cow(cowfile, &root.join("code/specs/cows"));

    // Replace placeholders
    cow_template = cow_template.replace("$eyes", &eyes);
    cow_template = cow_template.replace("$tongue", &tongue);
    cow_template = cow_template.replace("$thoughts", thoughts);

    // Final unescape
    let cow = cow_template.replace("\\\\", "\\");

    Some(format!("{}\n{}", bubble, cow))
}

fn random_message() -> String {
    let idx = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0)
        % RANDOM_MESSAGES.len();
    RANDOM_MESSAGES[idx].to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// PNG scene description — deliberately NOT platform-gated
// ═══════════════════════════════════════════════════════════════════════════
//
// Only the *rendering* of the PNG needs Metal and CoreText, and that part is
// gated to Apple below. Describing the scene is plain data, so the two helpers
// here stay un-gated even though the binary only calls them on macOS.
//
// That is not an accident, it is the fix for a real outage. `layout_ir` gained
// a new `TextContent` field (`wrap`); every construction site in the repo was
// updated except this one, because it sat inside `#[cfg(target_vendor =
// "apple")]` and therefore did not exist for the Linux and Windows compilers.
// The Ubuntu and Windows CI legs went green on a program that could not
// compile at all, and the breakage only surfaced much later on a macOS runner.
//
// Keeping the struct literal out here means all three CI legs type-check it.
// `dead_code` is allowed on non-Apple targets (the binary genuinely never calls
// it there) but a lint allowance does not stop type checking: the next time a
// field is added to `TextContent`, Ubuntu fails in minutes instead of macOS
// failing in months.

/// Monospace font for the PNG path. Menlo is chosen because cowsay's art only
/// aligns in a fixed-width face; `line_height` is tightened from layout-ir's
/// 1.2 default so consecutive rows of the cow touch the way a terminal's do.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
fn png_font() -> FontSpec {
    let mut font = font_spec("Menlo", 18.0);
    font.line_height = 1.1;
    font
}

/// Wrap cowsay's finished ASCII art in a `TextContent` for the layout pipeline.
#[cfg_attr(not(target_vendor = "apple"), allow(dead_code))]
fn cowsay_text_content(output: &str, font: FontSpec) -> TextContent {
    TextContent {
        value: output.to_string(),
        font,
        color: color_black(),
        decoration: None,
        max_lines: None,
        // Cowsay output is fixed-format monospace ASCII art: the speech
        // bubble's `/ \ | _ -` borders and the cow's legs only line up
        // because every line keeps exactly the spaces it was built with.
        // Soft wrapping is a paragraph feature — layout-to-paint's greedy
        // wrapper re-splits on `split_whitespace()`, which collapses runs
        // of spaces — so enabling it here could silently re-flow the art
        // into gibberish. Hard `\n` breaks (the only ones cowsay emits)
        // are preserved either way, so `false` is the faithful choice.
        wrap: false,
        text_align: TextAlign::Start,
    }
}

#[cfg(target_vendor = "apple")]
fn render_cowsay_png_metal(output: &str, path: &Path) -> Result<(), String> {
    let padding = 24.0;
    let font = png_font();
    let font_size = font.size;

    let measurer = NativeMeasurer::new();
    let lines: Vec<&str> = output.lines().collect();
    let line_count = lines.len().max(1) as f64;
    let max_text_width = lines
        .iter()
        .map(|line| measurer.measure(line, &font, None).width)
        .fold(0.0, f64::max);
    let line_height = measurer.measure("M", &font, None).height;

    let text_width = (max_text_width + font_size).ceil();
    let text_height = (line_height * line_count).ceil();
    let scene_width = (text_width + padding * 2.0).ceil().max(1.0);
    let scene_height = (text_height + padding * 2.0).ceil().max(1.0);

    let root = PositionedNode {
        x: padding,
        y: padding,
        width: text_width,
        height: text_height,
        id: Some("cowsay-text".to_string()),
        content: Some(Content::Text(cowsay_text_content(output, font))),
        children: Vec::new(),
        ext: HashMap::new(),
    };

    let resolver = NativeResolver::new();
    let metrics = NativeMetrics::new();
    let shaper = NativeShaper::new();

    let _: &dyn FontResolver<Handle = _> = &resolver;
    let _: &dyn FontMetrics<Handle = _> = &metrics;
    let _: &dyn TextShaper<Handle = _> = &shaper;

    let options = LayoutToPaintOptions {
        width: scene_width,
        height: scene_height,
        background: color_white(),
        device_pixel_ratio: 1.0,
        shaper: &shaper,
        metrics: &metrics,
        resolver: &resolver,
    };
    let scene = layout_to_paint(&root, &options);
    let pixels = paint_metal::render(&scene);

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("could not create {}: {}", parent.display(), e))?;
    }
    paint_codec_png::write_png(&pixels, &path.to_string_lossy())
        .map_err(|e| format!("could not write {}: {}", path.display(), e))
}

#[cfg(not(target_vendor = "apple"))]
fn render_cowsay_png_metal(_output: &str, _path: &Path) -> Result<(), String> {
    Err("paint-metal PNG rendering requires an Apple target".to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════
//
// These run on Linux, Windows and macOS alike. That is the point: the bug they
// guard against was a `TextContent` field that only the macOS compiler ever
// saw. Anything asserted here is asserted by all three CI legs.

#[cfg(test)]
mod tests {
    use super::*;

    /// The cow, rendered. Note the deliberate runs of interior spaces — this
    /// is the exact shape that soft wrapping would destroy.
    const SAMPLE_ART: &str = concat!(
        " _____\n",
        "< moo >\n",
        " -----\n",
        "        \\   ^__^\n",
        "         \\  (oo)\\_______\n",
        "            (__)\\       )\\/\\\n",
        "                ||----w |\n",
        "                ||     ||\n",
    );

    #[test]
    fn png_font_is_monospace_and_tightly_leaded() {
        let font = png_font();
        // Menlo is fixed-width; a proportional face would break every column
        // of the art regardless of what the wrapper does.
        assert_eq!(font.family, "Menlo");
        assert_eq!(font.size, 18.0);
        // Tighter than layout-ir's 1.2 default, so rows of the cow meet.
        assert_eq!(font.line_height, 1.1);
    }

    /// Regression test for the macOS-only build break.
    ///
    /// `layout_ir::TextContent` grew a `wrap` field. Every construction site
    /// in the repo was updated except cowsay's, which lived behind
    /// `#[cfg(target_vendor = "apple")]` and so was invisible to the Linux and
    /// Windows compilers — the failure surfaced only on a macOS runner, long
    /// after the change that caused it.
    ///
    /// Beyond re-asserting the value, the mere existence of this test forces
    /// the struct literal to be compiled on every platform.
    #[test]
    fn ascii_art_never_soft_wraps() {
        let content = cowsay_text_content(SAMPLE_ART, png_font());
        // Soft wrapping re-splits on `split_whitespace()`, collapsing the runs
        // of spaces that hold the cow together. Cowsay has already decided
        // where its line breaks go; the layout engine must not second-guess.
        assert!(!content.wrap, "cowsay ASCII art must never soft-wrap");
        // `max_lines` truncates once wrapping is on; unlimited is the only
        // correct answer for art whose height is fixed by its own newlines.
        assert_eq!(content.max_lines, None);
    }

    #[test]
    fn text_content_preserves_the_art_byte_for_byte() {
        let content = cowsay_text_content(SAMPLE_ART, png_font());
        // Nothing between cowsay and the layout engine may trim, re-indent or
        // re-flow the art: the value handed over is the value produced.
        assert_eq!(content.value, SAMPLE_ART);
        assert!(content.value.contains("        \\   ^__^"));
        assert_eq!(content.value.lines().count(), 8);
    }

    #[test]
    fn text_is_left_aligned_and_black() {
        let content = cowsay_text_content(SAMPLE_ART, png_font());
        // Centring or justifying pre-formatted art would shift whole rows
        // relative to each other.
        assert_eq!(content.text_align, TextAlign::Start);
        assert_eq!(content.color, color_black());
        assert_eq!(content.font, png_font());
    }

    #[test]
    fn empty_output_still_produces_valid_content() {
        let content = cowsay_text_content("", png_font());
        assert_eq!(content.value, "");
        assert!(!content.wrap);
    }

    #[test]
    fn non_apple_targets_report_png_rendering_as_unsupported() {
        // The stub and the real implementation must agree on their signature;
        // this pins the non-Apple contract so the two cannot drift apart.
        #[cfg(not(target_vendor = "apple"))]
        {
            let err = render_cowsay_png_metal("moo", Path::new("out.png"))
                .expect_err("PNG rendering cannot work without Metal");
            assert!(err.contains("Apple target"), "unexpected message: {err}");
        }
    }

    // ── Cow-file selection: path traversal (issue #12169) ──────────────────
    //
    // Each test builds a throwaway tree like this:
    //
    //   <tmp>/
    //     secret.cow          ← must NEVER be readable via -f
    //     cows/
    //       default.cow       ← the fallback
    //       tux.cow           ← a legitimate cow
    //       nested/inner.cow  ← exists, but "nested/inner" is not a bare name
    //
    // and asserts that every hostile spelling of "secret" draws the default cow.

    struct CowTree {
        base: PathBuf,
        cows: PathBuf,
    }

    impl CowTree {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let base = env::temp_dir().join(format!(
                "cowsay-traversal-{}-{}-{}",
                tag,
                std::process::id(),
                nanos
            ));
            let cows = base.join("cows");
            fs::create_dir_all(cows.join("nested")).unwrap();
            fs::write(
                cows.join("default.cow"),
                "$the_cow = <<EOC;\nDEFAULT\nEOC\n",
            )
            .unwrap();
            fs::write(cows.join("tux.cow"), "$the_cow = <<EOC;\nTUX\nEOC\n").unwrap();
            fs::write(
                cows.join("nested").join("inner.cow"),
                "$the_cow = <<EOC;\nNESTED\nEOC\n",
            )
            .unwrap();
            fs::write(base.join("secret.cow"), "$the_cow = <<EOC;\nSECRET\nEOC\n").unwrap();
            CowTree { base, cows }
        }
    }

    impl Drop for CowTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn safe_names_are_bare_file_stems() {
        for ok in [
            "default",
            "tux",
            "bud-frogs",
            "three_eyes",
            "v2",
            "dragon.and.cow",
        ] {
            assert!(is_safe_cow_name(ok), "should accept {ok:?}");
        }
        for bad in [
            "",
            "..",
            "../secret",
            "..\\secret",
            "a/b",
            "a\\b",
            "/etc/passwd",
            "C:secret",
            "C:\\Windows\\win",
            "tux\0",
            "..%2Fsecret",
            "%2e%2e/secret",
        ] {
            assert!(!is_safe_cow_name(bad), "should reject {bad:?}");
        }
    }

    #[test]
    fn normal_cow_names_still_load() {
        let tree = CowTree::new("normal");
        assert_eq!(load_cow("tux", &tree.cows), "TUX\n");
        assert_eq!(load_cow("default", &tree.cows), "DEFAULT\n");
    }

    #[test]
    fn unknown_cow_falls_back_to_default() {
        let tree = CowTree::new("missing");
        assert_eq!(load_cow("does-not-exist", &tree.cows), "DEFAULT\n");
    }

    #[test]
    fn relative_traversal_falls_back_to_default() {
        let tree = CowTree::new("relative");
        // Sanity: the target really is reachable by naive joining.
        assert!(tree.cows.join("../secret.cow").exists());
        for hostile in ["../secret", "..\\secret", "./../secret", "tux/../../secret"] {
            assert_eq!(
                load_cow(hostile, &tree.cows),
                "DEFAULT\n",
                "for {hostile:?}"
            );
        }
    }

    #[test]
    fn absolute_path_falls_back_to_default() {
        let tree = CowTree::new("absolute");
        let absolute = tree.base.join("secret");
        let absolute = absolute.to_str().unwrap();
        assert_eq!(load_cow(absolute, &tree.cows), "DEFAULT\n");
    }

    #[test]
    fn nested_names_are_refused_even_inside_the_cows_dir() {
        let tree = CowTree::new("nested");
        assert_eq!(load_cow("nested/inner", &tree.cows), "DEFAULT\n");
        assert_eq!(load_cow("nested\\inner", &tree.cows), "DEFAULT\n");
    }

    #[test]
    fn encoded_and_nul_names_fall_back_to_default() {
        let tree = CowTree::new("encoded");
        for hostile in [
            "..%2Fsecret",
            "%2e%2e%2fsecret",
            "%2E%2E%5Csecret",
            "tux\0../secret",
        ] {
            assert_eq!(
                load_cow(hostile, &tree.cows),
                "DEFAULT\n",
                "for {hostile:?}"
            );
        }
    }

    /// Layer 2 in action: the name `evil` is syntactically fine, but the file
    /// it names is a symlink leading out of the cows directory.
    #[cfg(unix)]
    #[test]
    fn symlink_escaping_the_cows_dir_falls_back_to_default() {
        let tree = CowTree::new("symlink");
        std::os::unix::fs::symlink(tree.base.join("secret.cow"), tree.cows.join("evil.cow"))
            .unwrap();
        assert_eq!(load_cow("evil", &tree.cows), "DEFAULT\n");
    }
}
