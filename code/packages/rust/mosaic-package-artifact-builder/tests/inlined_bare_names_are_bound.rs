//! Inlined bare names are bound, and bound to the right thing, on every
//! backend (UI29 §3.4, backlog 6f1m).
//!
//! A bare NAME in a package layout means the nearest enclosing `For`
//! binding, else the component's own slot. When a consumer mounts the
//! component, the resolver inlines it, and two things can go wrong:
//!
//! - an own-slot bare name (`Text ( content: heading )`) is left as the
//!   callee's slot name, which the consumer never declares: React
//!   `{heading}` fails `tsc`, SwiftUI fails `swiftc`, Web Components throws
//!   a `ReferenceError` (the original 6f1m report, fixed in the resolver by
//!   F1, #18393);
//! - a consumer name substituted into the body of a callee `For` that binds
//!   the same name is captured: the consumer's `row` silently reads the
//!   callee's loop variable. Every backend compiles that; every backend
//!   shows the wrong value.
//!
//! The fixture has both, plus a callee loop that legitimately shadows an
//! own slot and children spliced into a mount inside a callee loop. Each
//! backend's output is checked for the identifiers it must contain, and
//! where `tsc` or `swiftc` is installed the React or SwiftUI output is
//! type-checked, so an unbound name fails the test outright.

use std::fs;
use std::path::Path;
use std::process::Command;

use mosaic_package_artifact_builder::{compose_component, ComposedComponent};

/// `Card`, the dependency:
///
/// - `heading` is read bare outside any loop: Card's own slot;
/// - inside `For ( as: heading )` the bare `heading` is the loop variable;
/// - inside `For ( as: row )`, `title` is read bare. The consumer binds it
///   to *its* loop variable `row`, which Card's `row` must not capture;
/// - the children mount sits inside that loop, and the consumer's children
///   read the consumer's `row` too.
const CARD_MIL: &str = "component Card {
  slot heading : text ;
  slot title : text ;
  slot rows : list<text> ;
  slot tags : list<text> ;
  slot children : list<node> ;
}";

const CARD_MLL: &str = r#"layout Card { Column [ card ] {
  Text [ card-heading ] ( content: heading )
  For ( each: slot: tags , as: heading ) { Text [ card-tag ] ( content: heading ) }
  For ( each: slot: rows , as: row ) {
    Text [ card-title ] ( content: title )
    Text [ card-row ] ( content: row )
    Box [ card-slot ] { slot: children }
  }
} }"#;

const APP_MIL: &str = "component App { slot items : list<text> ; }";

const APP_MLL: &str = r#"layout App { Column [ app ] {
  For ( each: slot: items , as: row ) {
    pkg::mosaic-pkg-lib::Card ( heading: "Inbox" , title: row , rows: slot: items , tags: slot: items ) {
      Text [ app-row ] ( content: row )
    }
  }
} }"#;

fn write_package(root: &Path) {
    let lib = root.join("mosaic-pkg-lib");
    fs::create_dir_all(lib.join("src")).unwrap();
    fs::write(
        lib.join("mosaic-package.toml"),
        r#"[package]
name = "mosaic-pkg-lib"
version = "0.1.0"
description = "6f1m fixture"
license = "MIT"

[components]
exports = ["Card"]

[dependencies]

[kernel]
version = "1"
"#,
    )
    .unwrap();
    fs::write(lib.join("src/Card.mil"), CARD_MIL).unwrap();
    fs::write(lib.join("src/Card.mll"), CARD_MLL).unwrap();
}

fn composed() -> ComposedComponent {
    let tmp = tempfile::TempDir::new().unwrap();
    write_package(tmp.path());
    compose_component(
        "App",
        APP_MIL,
        APP_MLL,
        "style App { }",
        &[tmp.path().to_path_buf()],
        None,
    )
    .expect("App composes with Card inlined")
}

/// Every needle is in `out`, and no forbidden one is.
fn check(backend: &str, out: &str, needles: &[&str], forbidden: &[&str]) {
    for needle in needles {
        assert!(
            out.contains(needle),
            "{backend}: missing `{needle}`:\n{out}"
        );
    }
    for needle in forbidden {
        assert!(!out.contains(needle), "{backend}: has `{needle}`:\n{out}");
    }
}

#[test]
fn react_binds_every_inlined_name() {
    let c = composed();
    let out =
        mosaic_emit_react::pipeline::from_pipeline(&c.model.component, &c.layout.def, &c.style)
            .expect("react")
            .output;
    check(
        "react",
        &out,
        &[
            // own slot, substituted
            r#"<span>{"Inbox"}</span>"#,
            // the loop that shadows the own slot keeps its name and reads it
            "items.map((heading, _idx)",
            "<span>{heading}</span>",
            // Card's capturing loop is renamed; both rows are distinct
            "items.map((rowIn1, _idx)",
            "<span>{row}</span>",
            "<span>{rowIn1}</span>",
        ],
        &["{title}"],
    );
    typecheck_react(&out);
}

#[test]
fn swiftui_binds_every_inlined_name() {
    let c = composed();
    let out = mosaic_emit_swiftui::from_pipeline(&c.model.component, &c.layout.def, &c.style)
        .expect("swiftui")
        .output;
    check(
        "swiftui",
        &out,
        &[
            r#"Text("Inbox")"#,
            "{ heading in",
            "_mosaicText(heading)",
            "{ rowIn1 in",
            "_mosaicText(row)",
            "_mosaicText(rowIn1)",
        ],
        &["_mosaicText(title)"],
    );
    typecheck_swift(&out);
}

#[test]
fn compose_binds_every_inlined_name() {
    let c = composed();
    let out = mosaic_emit_compose::from_pipeline(&c.model.component, &c.layout.def, &c.style)
        .expect("compose")
        .output;
    check(
        "compose",
        &out,
        &[
            r#"Text(text = "Inbox")"#,
            "items.forEach { rowIn1 ->",
            r#"Text(text = "$row")"#,
            r#"Text(text = "$rowIn1")"#,
        ],
        &["$title"],
    );
}

#[test]
fn flutter_binds_every_inlined_name() {
    let c = composed();
    let out = mosaic_emit_flutter::from_pipeline(&c.model.component, &c.layout.def, &c.style)
        .expect("flutter")
        .output;
    check(
        "flutter",
        &out,
        &[
            r#"Text("Inbox")"#,
            "items.map((rowIn1) =>",
            "Text(row.toString())",
            "Text(rowIn1.toString())",
        ],
        &["title.toString()"],
    );
}

#[test]
fn webcomponent_binds_every_inlined_name() {
    let c = composed();
    let out = mosaic_emit_webcomponent::from_pipeline(&c.model.component, &c.layout.def, &c.style)
        .expect("webcomponent")
        .output;
    check(
        "webcomponent",
        &out,
        &[
            "<span>Inbox</span>",
            "items.map((rowIn1, idx)",
            "escapeHtml(row)",
            "escapeHtml(rowIn1)",
        ],
        &["escapeHtml(title)"],
    );
}

#[test]
fn qt_binds_every_inlined_name() {
    let c = composed();
    let out = mosaic_emit_qt::from_pipeline(&c.model.component, &c.layout.def, &c.style)
        .expect("qt")
        .output;
    check(
        "qt",
        &out,
        &[
            r#"text: "Inbox""#,
            "property var rowIn1: modelData",
            "text: row\n",
            "text: rowIn1\n",
        ],
        &["text: title"],
    );
}

/// Whether `tool --version` runs. The compile checks below need the real
/// compiler; without one they say so and pass, since the textual checks
/// above already pin the names.
fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// `tsc --strict` over the React output, against a stub of the two things
/// it imports from React. An unbound name is error TS2304.
fn typecheck_react(source: &str) {
    if !have("tsc") {
        eprintln!("tsc not installed; React output not type-checked");
        return;
    }
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("App.tsx"), source).unwrap();
    fs::write(
        dir.path().join("react.d.ts"),
        r#"declare module "react" {
  const React: { Fragment: any };
  export default React;
}
declare namespace JSX {
  interface IntrinsicElements { [name: string]: any }
  interface IntrinsicAttributes { key?: unknown }
}
"#,
    )
    .unwrap();
    let result = Command::new("tsc")
        .current_dir(dir.path())
        .args([
            "--noEmit",
            "--strict",
            "--jsx",
            "react",
            "--target",
            "es2020",
            "--module",
            "esnext",
            "--moduleResolution",
            "bundler",
            "App.tsx",
            "react.d.ts",
        ])
        .output()
        .expect("run tsc");
    assert!(
        result.status.success(),
        "tsc rejected the inlined React output:\n{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

/// `swiftc -typecheck` over the SwiftUI output (macOS only).
fn typecheck_swift(source: &str) {
    if !cfg!(target_os = "macos") || !have("swiftc") {
        eprintln!("swiftc not available; SwiftUI output not type-checked");
        return;
    }
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("App.swift");
    fs::write(&path, source).unwrap();
    let result = Command::new("swiftc")
        .arg("-typecheck")
        .arg(&path)
        .output()
        .expect("run swiftc");
    assert!(
        result.status.success(),
        "swiftc rejected the inlined SwiftUI output:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
