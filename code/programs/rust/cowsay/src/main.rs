use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_file, Parser};
use layout_ir::{
    color_black, color_white, font_spec, Content, PositionedNode, TextAlign, TextContent,
    TextMeasurer,
};
use layout_text_measure_native::NativeMeasurer;
use layout_to_paint::{layout_to_paint, LayoutToPaintOptions};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use text_native::text_interfaces::{FontMetrics, FontResolver, TextShaper};
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
        if current_line.len() + word.len() + 1 <= width {
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

fn load_cow(cow_name: &str, root: &Path) -> String {
    let mut cow_path = root.join(format!("code/specs/cows/{}.cow", cow_name));
    if !cow_path.exists() {
        cow_path = root.join("code/specs/cows/default.cow");
    }

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
    let mut cow_template = load_cow(cowfile, root);

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

#[cfg(target_vendor = "apple")]
fn render_cowsay_png_metal(output: &str, path: &Path) -> Result<(), String> {
    let font_size = 18.0;
    let padding = 24.0;
    let mut font = font_spec("Menlo", font_size);
    font.line_height = 1.1;

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
        content: Some(Content::Text(TextContent {
            value: output.to_string(),
            font,
            color: color_black(),
            max_lines: None,
            text_align: TextAlign::Start,
        })),
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
