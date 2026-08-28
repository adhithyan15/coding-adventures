use std::path::PathBuf;

fn main() {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/venture-visuals"));
    let capture = venture_browser_visual_fixtures::capture("http://venture.test")
        .expect("capture Venture visual fixture");
    capture.assert_valid();
    capture.write_pngs(&output).expect("write fixture PNGs");
    println!("{}", capture.describe());
    println!("wrote screenshots to {}", output.display());
}
