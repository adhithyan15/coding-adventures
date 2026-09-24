//! A long `If`/`Else` chain must type-check in reasonable time.
//!
//! App shells switch views with a nested chain:
//!
//! ```text
//! If (a-mode) { … } Else { If (b-mode) { … } Else { If (c-mode) { … } Else { … } } }
//! ```
//!
//! `If` lowers to an immediately-invoked closure. Swift type-checks a
//! multi-statement closure *together with* the expression around it
//! (SE-0326), so when a branch's `if`/`else` sat directly in the closure, every
//! level made the solver's problem larger. Trestle's seventh view (Checklists,
//! C3a of #14018) tipped its SwiftUI release build over the edge:
//!
//! ```text
//! TaskApp.swift:3983:141: error: the compiler is unable to type-check this
//! expression in reasonable time
//! ```
//!
//! The lowering now puts the branch inside a local `func _mosaicBranch()`,
//! as ordinary nodes already use `func _mosaicNode()`. A local function's body
//! is type-checked on its own, so the chain's cost no longer compounds.

#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::process::Command;

/// Views in the chain. Trestle has seven; this exceeds it.
const DEPTH: usize = 10;

/// Mosaic identifiers are letters and hyphens: `a`, `b`, … per level.
fn tag(level: usize) -> char {
    (b'a' + level as u8) as char
}

fn emit_chain() -> String {
    let mut mil = String::from("component Chain {\n");
    for level in 0..DEPTH {
        let t = tag(level);
        mil.push_str(&format!(
            "  slot mode-{t} : text ;\n  slot label-{t} : text ;\n"
        ));
    }
    mil.push_str("  emit onPick ;\n}\n");

    // Innermost first: the final Else, then each If wraps what came before.
    let mut body = String::from(
        "Column [ fallback ] { Text [ fallback-text ] ( content : \"Nothing selected\" ) }",
    );
    for level in (0..DEPTH).rev() {
        let t = tag(level);
        body = format!(
            "If ( when: slot: mode-{t} ) {{
  Column [ view-{t} ] {{
    Text [ title-{t} ] ( content : slot: label-{t} , a11y-role : heading )
    Row [ actions-{t} ] {{
      HostButton [ pick-{t} ] ( label : slot: label-{t} , onClick : emit: onPick )
      Text [ note-{t} ] ( content : \"Level {level}\" )
    }}
  }}
}}
Else {{
{body}
}}"
        );
    }
    let mll = format!("layout Chain {{\n  Column [ root ] {{\n{body}\n  }}\n}}\n");

    let mut msl = String::from("style Chain {\n");
    for level in 0..DEPTH {
        let t = tag(level);
        msl.push_str(&format!(
            "  part view-{t} {{ gap : 8 ; padding : 12 ; }}\n  part title-{t} {{ font-size : 16 ; color : \"#222222\" ; }}\n  part pick-{t} {{ padding : 6 ; background : \"#0d6efd\" ; color : \"#ffffff\" ; }}\n"
        ));
    }
    msl.push_str("}\n");

    let interface = mosmodel_compiler::compile(&mil).expect("compile chain interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile chain layout");
    let style =
        mosstyle_compiler::compile(&msl, Some(&layout.part_map_json)).expect("compile chain style");
    mosaic_emit_swiftui::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit chain SwiftUI")
        .output
}

#[test]
fn each_if_branch_is_its_own_local_function() {
    let out = emit_chain();
    assert_eq!(
        out.matches("func _mosaicBranch() -> AnyView").count(),
        DEPTH,
        "one local function per If"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_long_if_else_chain_typechecks() {
    let source_path =
        std::env::temp_dir().join(format!("mosaic-if-chain-{}.swift", std::process::id()));
    fs::write(&source_path, emit_chain()).expect("write generated Swift fixture");
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&source_path)
        .output()
        .expect("run swiftc");
    let _ = fs::remove_file(&source_path);
    assert!(
        result.status.success(),
        "a {DEPTH}-deep If/Else chain must typecheck:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
