use coding_adventures_warrant_ir::rust_subset::*;

#[test]
fn test_parse_ct_compare_source() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let result = parse_rust_file(source);
    let file = result.expect("Failed to parse ct-compare/src/lib.rs");

    assert_eq!(file.inner_attrs.len(), 2);
    assert_eq!(file.inner_attrs[0].name, "deny");
    assert_eq!(file.inner_attrs[0].args, vec!["unsafe_code"]);
    assert_eq!(file.inner_attrs[1].name, "deny");
    assert_eq!(file.inner_attrs[1].args, vec!["unsafe_op_in_unsafe_fn"]);

    // Find volatile_barrier module
    let mut found_volatile_barrier = false;
    let mut found_capabilities = false;
    let mut found_ct_eq = false;
    let mut found_ct_eq_fixed = false;
    let mut found_ct_select_bytes = false;
    let mut found_ct_eq_u64 = false;

    for item in &file.items {
        match item {
            Item::Module(m) if m.name == "capabilities" => {
                found_capabilities = true;
                assert_eq!(m.vis, Visibility::Public);
                assert!(m.items.is_empty());
            }
            Item::Module(m) if m.name == "volatile_barrier" => {
                found_volatile_barrier = true;
                assert_eq!(m.items.len(), 3); // use core::ptr, fn accumulate, fn read
            }
            Item::Function(f) if f.name == "ct_eq" => {
                found_ct_eq = true;
                assert_eq!(f.vis, Visibility::Public);
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.return_type, Some(RustType::Bool));
                assert!(f.is_inline_never);
            }
            Item::Function(f) if f.name == "ct_eq_fixed" => {
                found_ct_eq_fixed = true;
                assert_eq!(f.vis, Visibility::Public);
                assert_eq!(f.const_generics.len(), 1);
                assert_eq!(f.const_generics[0].name, "N");
                assert_eq!(f.const_generics[0].ty, RustType::Usize);
            }
            Item::Function(f) if f.name == "ct_select_bytes" => {
                found_ct_select_bytes = true;
                assert_eq!(f.vis, Visibility::Public);
                assert_eq!(f.params.len(), 3);
                assert_eq!(f.return_type, Some(RustType::Vec(Box::new(RustType::U8))));
            }
            Item::Function(f) if f.name == "ct_eq_u64" => {
                found_ct_eq_u64 = true;
                assert_eq!(f.vis, Visibility::Public);
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.return_type, Some(RustType::Bool));
            }
            _ => {}
        }
    }

    assert!(found_volatile_barrier);
    assert!(found_capabilities);
    assert!(found_ct_eq);
    assert!(found_ct_eq_fixed);
    assert!(found_ct_select_bytes);
    assert!(found_ct_eq_u64);
}

#[test]
fn test_lexer_tokens_and_literals() {
    let code = r#"
        fn foo() {
            let x: u8 = 42;
            let y: u64 = 0xDEAD_BEEF;
            let z: u32 = 0b1010_0101;
            let s: &str = "hello\n\t\"\\";
            let b: &[u8] = b"byte\x41\0\r\n\t\\\"";
            let b_empty: &[u8] = b"";
            let bool_val = true && false;
        }
    "#;
    let file = parse_rust_file(code).expect("Should parse literals");
    assert_eq!(file.items.len(), 1);
    if let Item::Function(f) = &file.items[0] {
        assert_eq!(f.name, "foo");
        assert_eq!(f.body.stmts.len(), 7);
    } else {
        panic!("Expected function foo");
    }
}

#[test]
fn test_operators_and_expressions() {
    let code = r#"
        pub fn ops(a: u32, b: u32) -> u32 {
            let c = a + b - a * b / a % b;
            let d = (a & b) | (a ^ b) << 2 >> 1;
            let cmp = a == b || a != b && a < b && a <= b && a > b && a >= b;
            let mut x = 10;
            x += 1;
            x -= 1;
            x |= 2;
            x &= 3;
            x ^= 4;
            x <<= 1;
            x >>= 1;
            x = 0;
            return x;
        }
    "#;
    let file = parse_rust_file(code).expect("Should parse operators");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_control_flow_and_loops() {
    let code = r#"
        fn control_flow(cond: bool, n: usize) -> u8 {
            if cond {
                return 1;
            } else if n > 10 {
                return 2;
            } else {
                return 0;
            }

            for i in 0..n {
                let _ = i;
            }

            for j in 0..=n {
                let _ = j;
            }

            unsafe {
                let ptr = 0;
            }
        }
    "#;
    let file = parse_rust_file(code).expect("Should parse control flow");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_use_declarations() {
    let code = r#"
        use core::hint::black_box;
        use core::ptr::{read_volatile, write_volatile};
        use super::*;
        use a::b::c;
    "#;
    let file = parse_rust_file(code).expect("Should parse use declarations");
    assert_eq!(file.items.len(), 4);
    match &file.items[0] {
        Item::Use(u) => {
            assert_eq!(u.path, vec!["core", "hint"]);
            assert_eq!(u.tree, UseTree::Simple("black_box".to_string()));
        }
        _ => panic!("Expected Item::Use"),
    }
    match &file.items[1] {
        Item::Use(u) => {
            assert_eq!(u.path, vec!["core", "ptr"]);
            match &u.tree {
                UseTree::Group(g) => assert_eq!(g.len(), 2),
                _ => panic!("Expected UseTree::Group"),
            }
        }
        _ => panic!("Expected Item::Use"),
    }
    match &file.items[2] {
        Item::Use(u) => {
            assert_eq!(u.path, vec!["super"]);
            assert_eq!(u.tree, UseTree::Wildcard);
        }
        _ => panic!("Expected Item::Use"),
    }
}

#[test]
fn test_arrays_and_indexing() {
    let code = r#"
        fn arrays() {
            let empty: [u8; 0] = [];
            let list = [1, 2, 3, 4];
            let repeat = [0u8; 32];
            let elem = list[0];
            let ref_elem = &repeat[1];
        }
    "#;
    let file = parse_rust_file(code).expect("Should parse arrays");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_attributes_and_visibility() {
    let code = r#"
        #![deny(warnings)]
        #![allow(unused)]

        #[inline(always)]
        pub fn pub_fn() {}

        #[inline(never)]
        pub(super) fn super_fn() {}

        #[allow(unsafe_code)]
        fn priv_fn() {}
    "#;
    let file = parse_rust_file(code).expect("Should parse attributes");
    assert_eq!(file.inner_attrs.len(), 2);
    assert_eq!(file.items.len(), 3);
    if let Item::Function(f) = &file.items[0] {
        assert_eq!(f.vis, Visibility::Public);
        assert!(f.is_inline_always);
    }
    if let Item::Function(f) = &file.items[1] {
        assert_eq!(f.vis, Visibility::PublicSuper);
        assert!(f.is_inline_never);
    }
    if let Item::Function(f) = &file.items[2] {
        assert_eq!(f.vis, Visibility::Private);
    }
}

#[test]
fn test_comments_and_whitespace() {
    let code = r#"
        // Line comment 1
        // Line comment 2
        /* Block comment */
        /* Multi-line
           block
           comment */
        /* Nested /* block */ comment */
        fn with_comments() {}
    "#;
    let file = parse_rust_file(code).expect("Should parse comments");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_types_parsing() {
    let code = r#"
        fn types(
            a: u8, b: u16, c: u32, d: u64, e: usize,
            f: i8, g: i16, h: i32, i: i64, j: isize,
            k: bool, l: (), m: &u8, n: &mut u8,
            o: &[u8], p: [u8; 16], q: [u8; N],
            r: Vec<u8>, s: CustomType
        ) {}
    "#;
    let file = parse_rust_file(code).expect("Should parse all types");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_macros_and_calls() {
    let code = r#"
        fn test_macros() {
            assert!(true);
            assert_eq!(1, 1);
            debug_assert_eq!(2, 2);
            let v = vec![0u8; 32];
            let w = vec![1, 2, 3];
            let call = some_fn(1, 2);
            let method = obj.method(3, 4);
            let chained = obj.m1().m2();
            let cast = x as u64;
            let deref = *ptr;
            let neg = -num;
            let not = !flag;
        }
    "#;
    let file = parse_rust_file(code).expect("Should parse macros and calls");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parser_and_lexer_error_paths() {
    // 1. Source too large
    let huge = " ".repeat(1_048_577);
    assert!(parse_rust_file(&huge).is_err());

    // 2. Unterminated block comment
    assert!(parse_rust_file("/* unterminated").is_err());

    // 3. Unterminated string
    assert!(parse_rust_file(r#""unterminated"#).is_err());

    // 4. Unterminated byte string
    assert!(parse_rust_file(r#"b"unterminated"#).is_err());

    // 5. Invalid hex in byte string
    assert!(parse_rust_file(r#"b"\xZZ""#).is_err());

    // 6. Invalid escape in byte string
    assert!(parse_rust_file(r#"b"\q""#).is_err());

    // 7. Unexpected byte
    assert!(parse_rust_file("@").is_err());

    // 8. Max recursion depth exceeded
    let deeply_nested = "(".repeat(35) + "1" + &")".repeat(35);
    assert!(parse_rust_file(&format!("fn f() {{ {} }}", deeply_nested)).is_err());

    // 9. Malformed attribute
    assert!(parse_rust_file("#[123]").is_err());
    assert!(parse_rust_file("#[attr(123)]").is_err());

    // 10. Unsupported visibility
    assert!(parse_rust_file("pub(crate) fn f() {}").is_err());

    // 11. Malformed item
    assert!(parse_rust_file("struct Foo;").is_err());

    // 12. Malformed const generic
    assert!(parse_rust_file("fn f<123>() {}").is_err());
    assert!(parse_rust_file("fn f<const 123: usize>() {}").is_err());

    // 13. Malformed param
    assert!(parse_rust_file("fn f(123: u8) {}").is_err());

    // 14. Malformed array size in type
    assert!(parse_rust_file("fn f(a: [u8; 'x']) {}").is_err());

    // 15. Malformed let
    assert!(parse_rust_file("fn f() { let 123 = 4; }").is_err());

    // 16. Malformed for
    assert!(parse_rust_file("fn f() { for 123 in 0..10 {} }").is_err());

    // 17. Malformed use
    assert!(parse_rust_file("use 123;").is_err());
    assert!(parse_rust_file("use foo::123;").is_err());
    assert!(parse_rust_file("use foo::{123};").is_err());

    // 18. Malformed mod
    assert!(parse_rust_file("mod 123 {}").is_err());

    // 19. Malformed method call
    assert!(parse_rust_file("fn f() { x.123(); }").is_err());

    // 20. Malformed macro delimiter
    assert!(parse_rust_file("fn f() { vec! <123>; }").is_err());

    // 21. Missing semicolon
    assert!(parse_rust_file("fn f() { 1 2 }").is_err());

    // 22. Deeply nested unary operators
    let deeply_unary = "!".repeat(35) + "true";
    assert!(parse_rust_file(&format!("fn f() {{ {} }}", deeply_unary)).is_err());

    // 23. Exceeded token limit
    let too_many_tokens = "0 ".repeat(65_537);
    assert!(parse_rust_file(&too_many_tokens).is_err());
}
