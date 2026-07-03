//! Generate a sample deck for an external python-pptx cross-check.
//!
//! ```sh
//! cargo run -p coding-adventures-pptx-writer --example gen_deck -- /tmp/sample.pptx
//! ```
use coding_adventures_pptx_writer::{write_pptx, Presentation};

fn main() {
    let mut p = Presentation::new();
    let s1 = p.add_slide();
    s1.add_text("Slide One Title");
    s1.add_text("First slide body");
    let s2 = p.add_slide();
    s2.add_text("Slide Two Title");
    s2.add_text("Second slide body with special & <chars>");

    let bytes = write_pptx(&p);
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sample.pptx".to_string());
    std::fs::write(&out, bytes).expect("write pptx");
    println!("wrote {out}");
}
