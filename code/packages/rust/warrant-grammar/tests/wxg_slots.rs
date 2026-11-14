//! VER04k section 4.2 and 4.11: the post-state slot resolver, differential-tested
//! against a brute-force enumerator over every result type of depth 2 built from
//! three base types, and the parsed shape of `inout` / `ensures`.

use coding_adventures_warrant_grammar::checker::{resolve_post_slot, PostSlot, SlotShape};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, Declaration, ParserLimits};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

const BASE: [char; 3] = ['A', 'B', 'C'];

/// Every result shape of depth 2 over the three base types: a plain type, a
/// tuple of 2 to 4 of them, and each of those wrapped in a `Result`.
fn all_shapes() -> Vec<SlotShape<char>> {
    let mut shapes = Vec::new();
    for &b in &BASE {
        shapes.push(SlotShape::Plain(b));
        shapes.push(SlotShape::ResultPlain(b));
    }
    for len in 2..=4usize {
        for n in 0..BASE.len().pow(len as u32) {
            let cs: Vec<char> = (0..len).map(|i| BASE[(n / BASE.len().pow(i as u32)) % BASE.len()]).collect();
            shapes.push(SlotShape::Tuple(cs.clone()));
            shapes.push(SlotShape::ResultTuple(cs));
        }
    }
    shapes
}

/// Every non-empty list of post-state types of length 1 to 4.
fn all_posts() -> Vec<Vec<char>> {
    let mut out = Vec::new();
    for k in 1..=4usize {
        for n in 0..BASE.len().pow(k as u32) {
            out.push((0..k).map(|i| BASE[(n / BASE.len().pow(i as u32)) % BASE.len()]).collect());
        }
    }
    out
}

/// The oracle, written differently from the resolver: it tries every way of
/// removing `k` components from the end of a tuple by building the candidate
/// host return and the post-state list and comparing them as strings.
fn brute_force(posts: &[char], shape: &SlotShape<char>) -> Option<PostSlot> {
    let k = posts.len();
    let want: String = posts.iter().collect();
    match shape {
        SlotShape::Plain(t) => (k == 1 && t.to_string() == want).then_some(PostSlot::S1),
        SlotShape::ResultPlain(t) => (k == 1 && t.to_string() == want).then_some(PostSlot::S2),
        SlotShape::Tuple(cs) | SlotShape::ResultTuple(cs) => {
            let rendered: String = cs.iter().collect();
            // n >= 2 holds by construction; n >= k and the last k are the posts.
            if cs.len() < k || !rendered.ends_with(&want) {
                return None;
            }
            // `ends_with` on the rendered string is the same test as on the
            // component list because every component is one character.
            let h = cs.len() - k;
            Some(if matches!(shape, SlotShape::Tuple(_)) {
                PostSlot::S3 { h }
            } else {
                PostSlot::S4 { h }
            })
        }
    }
}

#[test]
fn the_resolver_agrees_with_the_brute_force_enumerator_everywhere() {
    let (mut matched, mut total) = (0, 0);
    for posts in all_posts() {
        for shape in all_shapes() {
            let got = resolve_post_slot(&posts, &shape);
            assert_eq!(got, brute_force(&posts, &shape), "{posts:?} against {shape:?}");
            total += 1;
            matched += usize::from(got.is_some());
        }
    }
    // The universe is not degenerate: both outcomes occur, many times.
    assert!(matched > 100 && total - matched > 100, "{matched} of {total}");
}

#[test]
fn the_slot_names_are_the_corpus_names() {
    assert_eq!(PostSlot::S1.name(), "S1");
    assert_eq!(PostSlot::S2.name(), "S2");
    assert_eq!(PostSlot::S3 { h: 1 }.name(), "S3");
    assert_eq!(PostSlot::S4 { h: 2 }.name(), "S4");
}

#[test]
fn the_remaining_component_count_is_n_minus_k() {
    let s3 = resolve_post_slot(&['B', 'C'], &SlotShape::Tuple(vec!['A', 'B', 'C']));
    assert_eq!(s3, Some(PostSlot::S3 { h: 1 }));
    let s3_all = resolve_post_slot(&['A', 'B'], &SlotShape::Tuple(vec!['A', 'B']));
    assert_eq!(s3_all, Some(PostSlot::S3 { h: 0 }));
    let s4 = resolve_post_slot(&['C'], &SlotShape::ResultTuple(vec!['A', 'B', 'C']));
    assert_eq!(s4, Some(PostSlot::S4 { h: 2 }));
}

fn parse<R>(src: &str, f: impl FnOnce(&coding_adventures_warrant_grammar::parser::WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

#[test]
fn the_parsed_shape_marks_inout_parameters_and_keeps_the_ensures_rows() {
    let src = "warrant t v1\n  op f(a: U8, b: inout Bytes, c: inout Nat | c > 1) -> (Nat, Bytes, Nat)\n  ensures f:\n    len(result.1) == len(b)\n    result.0 == a\n";
    parse(src, |file| {
        let Declaration::Op(op) = &file.declarations[0] else { panic!("op") };
        let flags: Vec<bool> = op.params.iter().map(|p| p.inout).collect();
        assert_eq!(flags, [false, true, true]);
        assert!(op.params[2].refinement.is_some(), "the refinement binds to the pre-state");
        let Declaration::Ensures(e) = &file.declarations[1] else { panic!("ensures") };
        assert_eq!(e.op, "f");
        assert_eq!(e.rows.len(), 2);
    });
}

#[test]
fn an_alias_named_inout_is_not_a_mode() {
    parse("warrant t v1\n  model inout = U8\n  op f(x: inout) -> U8\n", |file| {
        let Declaration::Op(op) = &file.declarations[1] else { panic!("op") };
        assert!(!op.params[0].inout);
    });
}

#[test]
fn debug_output_of_a_parameter_without_inout_is_the_pre_wx_g_text() {
    // The two v0 specifications' hashes (tests/v0_compat.rs) are hashes of this text.
    parse("warrant t\n  op f(x: U8) -> U8\n", |file| {
        let Declaration::Op(op) = &file.declarations[0] else { panic!("op") };
        let text = format!("{:?}", op.params[0]);
        assert!(text.starts_with("Parameter { name: \"x\", param_type: Named"), "{text}");
        assert!(!text.contains("inout"), "{text}");
    });
    parse("warrant t v1\n  op f(x: inout U8) -> U8\n", |file| {
        let Declaration::Op(op) = &file.declarations[0] else { panic!("op") };
        assert!(format!("{:?}", op.params[0]).contains("inout: true"));
    });
}
