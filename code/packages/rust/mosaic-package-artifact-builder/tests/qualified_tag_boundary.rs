//! #14886 — every emitter rejects a qualified `pkg::P::C` tag.
//!
//! `LayoutPackageResolver` promises to hand backends "a layout tree
//! containing no qualified tags", and both `mosaic-compile` and the
//! artifact builder run it before emit. Every emitter is written against
//! that guarantee, and none of them calls `package_ref()` or
//! `component()` to check.
//!
//! That made the guarantee assumed rather than enforced. #14884 is the
//! worked example: a `.mll` with a qualified `Cell` reached the XAML
//! emitter with no resolver pass and `main` went red.
//!
//! The failure mode actually worth fearing is the QUIET one — a backend
//! that strips the qualifier and emits a structurally plausible element
//! for the wrong component. Measured across all eight, that does not
//! happen: each returns `UnknownPrimitive` naming the offending tag.
//!
//! This test pins that. It is the enforcement the contract was missing,
//! and it fails if any emitter ever starts accepting a qualified tag
//! silently.

use moslayout_compiler::{LayoutDef, LayoutNode};
use mosmodel_compiler::MosmodelComponent;
use mosstyle_compiler::StyleDef;

const QUALIFIED: &str = "pkg::mosaic-pkg-grid::Grid";

fn fixture() -> (MosmodelComponent, LayoutDef, StyleDef) {
    (
        MosmodelComponent {
            component: "X".to_string(),
            slots: vec![],
            emits: vec![],
        },
        LayoutDef {
            component_name: "X".to_string(),
            root: LayoutNode {
                tag: "Column".to_string(),
                part_name: None,
                props: vec![],
                children: vec![LayoutNode {
                    tag: QUALIFIED.to_string(),
                    part_name: None,
                    props: vec![],
                    children: vec![],
                }],
            },
        },
        StyleDef {
            component_name: "X".to_string(),
            parts: vec![],
        },
    )
}

/// The emitted source must never be produced. Asserting on the ERROR
/// rather than on absence of output is deliberate: a backend that
/// emitted an empty string would satisfy "no Grid in the output" while
/// having silently dropped the component.
fn assert_rejected(backend: &str, result: Result<String, String>) {
    match result {
        Err(message) => assert!(
            message.contains(QUALIFIED),
            "{backend} rejected the tag but did not name it: {message}"
        ),
        Ok(source) => panic!(
            "{backend} ACCEPTED a qualified tag and emitted {} bytes. The resolver \
             contract is now unenforced for this backend, and a stripped qualifier \
             emits a plausible element for the wrong component (#14886).\n{source}",
            source.len()
        ),
    }
}

#[test]
fn every_emitter_rejects_a_qualified_tag() {
    let (c, l, s) = fixture();
    assert_rejected(
        "compose",
        mosaic_emit_compose::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "swiftui",
        mosaic_emit_swiftui::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "html",
        mosaic_emit_html::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "react",
        mosaic_emit_react::pipeline::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "webcomponent",
        mosaic_emit_webcomponent::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "qt",
        mosaic_emit_qt::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "flutter",
        mosaic_emit_flutter::from_pipeline(&c, &l, &s)
            .map(|r| r.output)
            .map_err(|e| format!("{e:?}")),
    );
    assert_rejected(
        "xaml",
        mosaic_emit_xaml::pipeline::from_pipeline(
            &c,
            &l,
            &s,
            None,
            &mosaic_emit_xaml::EmitOptions::default(),
        )
            .map(|r| r.xaml)
            .map_err(|e| format!("{e:?}")),
    );
}
