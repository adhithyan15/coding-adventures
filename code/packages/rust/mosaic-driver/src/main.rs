//! # mosaic — The Mosaic compiler driver.
//!
//! `mosaic` is the CLI entry point that stitches together the three Mosaic
//! compiler stages into a single end-to-end pipeline:
//!
//! ```text
//! .mil  ──▶  mosmodel-compiler  ──▶  interface descriptor JSON
//!                                        │
//! .mll  ──▶  moslayout-compiler ◀────────┤  ──▶  part-map JSON
//!                                        │
//! .msl  ──▶  mosstyle-compiler  ◀────────┘  ──▶  CSS string
//! ```
//!
//! ## Usage
//!
//! ```text
//! mosaic <ComponentName>
//!   Compile <Name>.mil + <Name>.mll + <Name>.msl from the current directory.
//!
//! mosaic --interface  Grid.mil
//!   Run only the model stage, print descriptor JSON.
//!
//! mosaic --layout Grid.mll
//!   Run only the layout stage (no .mil needed), print part-map JSON.
//!
//! mosaic --style Grid.msl
//!   Run only the style stage (no .mil/.mll needed), print CSS.
//! ```
//!
//! ## Output (stdout JSON)
//!
//! ```json
//! {
//!   "component": "Grid",
//!   "interface": { "slots": [...], "emits": [...] },
//!   "parts": [{ "name": "root", "primitive": "Column" }, ...],
//!   "css": ".mos-Grid-root { background-color: #1e1e1e; ... }"
//! }
//! ```

use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: mosaic <ComponentName>");
        eprintln!("       mosaic --interface <path.mil>");
        eprintln!("       mosaic --layout    <path.mll>");
        eprintln!("       mosaic --style     <path.msl>");
        process::exit(1);
    }

    let flag = args[1].as_str();

    match flag {
        "--interface" => {
            // ── model stage only ─────────────────────────────────────────
            let path = require_arg(&args, 2, "--interface requires a .mil path");
            let src = read_file(&path);
            let out = mosmodel_compiler::compile(&src).unwrap_or_else(|errs| {
                for e in &errs { eprintln!("mosmodel error: {e}"); }
                process::exit(1);
            });
            println!("{}", out.descriptor_json);
        }

        "--layout" => {
            // ── layout stage only ────────────────────────────────────────
            // No interface descriptor: slot-ref validation is skipped.
            let path = require_arg(&args, 2, "--layout requires a .mll path");
            let src = read_file(&path);
            let out = moslayout_compiler::compile(&src, None).unwrap_or_else(|errs| {
                for e in &errs { eprintln!("moslayout error: {e}"); }
                process::exit(1);
            });
            println!("{}", out.part_map_json);
        }

        "--style" => {
            // ── style stage only ─────────────────────────────────────────
            // No part-map: part-name validation is skipped.
            let path = require_arg(&args, 2, "--style requires a .msl path");
            let src = read_file(&path);
            let out = mosstyle_compiler::compile(&src, None).unwrap_or_else(|errs| {
                for e in &errs { eprintln!("mosstyle error: {e}"); }
                process::exit(1);
            });
            println!("{}", out.css);
        }

        name => {
            // ── full three-stage pipeline ─────────────────────────────────
            //
            // Validate that `name` is a safe identifier before using it to
            // construct file paths.  Without this check a caller could pass
            // `../../etc/passwd` and read arbitrary files relative to the cwd.
            // We allow alphanumeric characters plus hyphens and underscores —
            // the same characters the Mosaic grammar permits in component names.
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                eprintln!(
                    "Invalid component name '{}': only alphanumeric characters, '-', and '_' are allowed",
                    name
                );
                process::exit(1);
            }

            // Expect <Name>.mil, <Name>.mll, <Name>.msl in the cwd.
            let mil_path = format!("{name}.mil");
            let mll_path = format!("{name}.mll");
            let msl_path = format!("{name}.msl");

            let mil_src = read_file(&mil_path);
            let mll_src = read_file(&mll_path);
            let msl_src = read_file(&msl_path);

            // ── Stage 1: model ────────────────────────────────────────────
            let model_out = mosmodel_compiler::compile(&mil_src).unwrap_or_else(|errs| {
                for e in &errs { eprintln!("mosmodel error in {mil_path}: {e}"); }
                process::exit(1);
            });

            // ── Stage 2: layout ───────────────────────────────────────────
            // Pass the descriptor JSON so the layout compiler can validate
            // slot references against the declared interface.
            let layout_out =
                moslayout_compiler::compile(&mll_src, Some(&model_out.descriptor_json))
                    .unwrap_or_else(|errs| {
                        for e in &errs { eprintln!("moslayout error in {mll_path}: {e}"); }
                        process::exit(1);
                    });

            // ── Stage 3: style ────────────────────────────────────────────
            // Pass the part-map JSON so the style compiler can validate that
            // every styled part was actually declared in the layout.
            let style_out =
                mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json))
                    .unwrap_or_else(|errs| {
                        for e in &errs { eprintln!("mosstyle error in {msl_path}: {e}"); }
                        process::exit(1);
                    });

            // ── Emit JSON summary ─────────────────────────────────────────
            // Deserialize descriptor_json and part_map_json so we can embed
            // them as structured JSON (not double-encoded strings).
            let interface: serde_json::Value =
                serde_json::from_str(&model_out.descriptor_json)
                    .unwrap_or(serde_json::Value::Null);
            let parts: serde_json::Value =
                serde_json::from_str(&layout_out.part_map_json)
                    .unwrap_or(serde_json::Value::Null);

            let summary = serde_json::json!({
                "component": name,
                "interface": interface,
                "parts":     parts,
                "css":       style_out.css,
            });

            match serde_json::to_string_pretty(&summary) {
                Ok(s)  => println!("{}", s),
                Err(e) => {
                    eprintln!("Internal error serialising output: {e}");
                    process::exit(1);
                }
            }
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Return `args[idx]` or print an error and exit.
fn require_arg(args: &[String], idx: usize, msg: &str) -> String {
    args.get(idx).cloned().unwrap_or_else(|| {
        eprintln!("{msg}");
        process::exit(1);
    })
}

/// Read a file to a String or print an error and exit.
fn read_file(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {path}: {e}");
        process::exit(1);
    })
}
