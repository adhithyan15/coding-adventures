use coding_adventures_ct_compare::*;
use coding_adventures_warrant_ir::rust_subset::parse_rust_file;
use coding_adventures_warrant_ir::*;

#[test]
fn test_lower_and_differential_ct_eq_u64() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_u64", &["a", "b"])]).expect("Failed to lower");

    let f = program.find_function("ct_eq_u64").expect("ct_eq_u64 not found");
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].1, WirType::U64);
    assert_eq!(f.params[0].2, Label::SECRET);

    let mut interp = Interpreter::new(&program);

    let test_cases: Vec<(u64, u64)> = vec![
        (0, 0),
        (u64::MAX, u64::MAX),
        (0xDEAD_BEEF, 0xDEAD_BEEF),
        (0, 1),
        (1, 0),
        (0x1234_5678, 0x8765_4321),
    ];

    for (a, b) in test_cases {
        let expected = ct_eq_u64(a, b);
        let outcome = interp.run("ct_eq_u64", &[WirValue::u64(a), WirValue::u64(b)]);
        assert_eq!(outcome.unwrap_value(), WirValue::Bool(expected));
    }

    // Sweep all 64 single-bit perturbations
    let base: u64 = 0x1234_5678_9ABC_DEF0;
    for bit in 0..64 {
        let flipped = base ^ (1u64 << bit);
        let expected = ct_eq_u64(base, flipped);
        let outcome = interp.run("ct_eq_u64", &[WirValue::u64(base), WirValue::u64(flipped)]);
        assert_eq!(outcome.unwrap_value(), WirValue::Bool(expected));
    }
}

#[test]
fn test_lower_and_differential_ct_eq_equal_length() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_equal_length", &["a", "b"])]).expect("Failed to lower");

    let mut interp = Interpreter::new(&program);

    let a_bytes = b"differential_testing_123456789";
    let b_bytes_identical = b"differential_testing_123456789";
    let b_bytes_diff_first = b"xifferential_testing_123456789";
    let b_bytes_diff_last = b"differential_testing_12345678x";

    let test_pairs = vec![
        (a_bytes.as_slice(), b_bytes_identical.as_slice()),
        (a_bytes.as_slice(), b_bytes_diff_first.as_slice()),
        (a_bytes.as_slice(), b_bytes_diff_last.as_slice()),
    ];

    for (left, right) in test_pairs {
        let expected = ct_eq(left, right);

        let p_left = interp.allocate_bytes(left).unwrap();
        let p_right = interp.allocate_bytes(right).unwrap();

        let outcome = interp.run("ct_eq_equal_length", &[p_left, p_right]);
        assert_eq!(outcome.unwrap_value(), WirValue::Bool(expected));
    }
}

#[test]
fn test_lower_and_differential_ct_select_bytes() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_select_bytes", &["a", "b", "choice"])]).expect("Failed to lower");

    let mut interp = Interpreter::new(&program);

    let a_bytes = b"AAAAAAAAAAAAAAAA";
    let b_bytes = b"BBBBBBBBBBBBBBBB";

    for choice in [true, false] {
        let expected = ct_select_bytes(a_bytes, b_bytes, choice);

        let p_a = interp.allocate_bytes(a_bytes).unwrap();
        let p_b = interp.allocate_bytes(b_bytes).unwrap();

        let outcome = interp.run(
            "ct_select_bytes",
            &[p_a, p_b, WirValue::Bool(choice)],
        );

        let p_out = outcome.unwrap_value();
        let result_bytes = interp.read_bytes(&p_out, a_bytes.len()).unwrap();
        assert_eq!(result_bytes, expected);
    }
}

#[test]
fn test_observable_trace_constant_time_guarantee() {
    let source = include_str!("../../ct-compare/src/lib.rs");
    let file = parse_rust_file(source).expect("Failed to parse ct-compare source");
    let program = lower_source_file(&file, &[("ct_eq_equal_length", &["a", "b"])]).expect("Failed to lower");
    if let Some(f) = program.find_function("ct_eq_equal_length") {
        for b in &f.blocks {
            println!("Block {:?}:", b.id);
            for inst in &b.instructions {
                println!("  {:?}", inst);
            }
            println!("  term: {:?}", b.terminator);
        }
    }

    // Run 1: identical inputs
    let mut interp1 = Interpreter::new(&program);
    let p_a1 = interp1.allocate_bytes(&[0x42; 16]).unwrap();
    let p_b1 = interp1.allocate_bytes(&[0x42; 16]).unwrap();
    let out1 = interp1.run("ct_eq_equal_length", &[p_a1, p_b1]);
    assert_eq!(out1.unwrap_value(), WirValue::Bool(true));

    // Run 2: differing inputs (different at first byte)
    let mut interp2 = Interpreter::new(&program);
    let diff_bytes = [0x99, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42];
    let p_a2 = interp2.allocate_bytes(&[0x42; 16]).unwrap();
    let p_b2 = interp2.allocate_bytes(&diff_bytes).unwrap();
    let out2 = interp2.run("ct_eq_equal_length", &[p_a2, p_b2]);
    assert_eq!(out2.unwrap_value(), WirValue::Bool(false));

    // CWE-208 CONSTANT-TIME PROOF OVER THE OBSERVABLE TRACE:
    // Both runs must have identical step counts, branch sequences, and memory access addresses!
    assert_eq!(interp1.trace().events.len(), interp2.trace().events.len());
    assert_eq!(interp1.cost().steps, interp2.cost().steps);
    assert_eq!(interp1.trace().events, interp2.trace().events);
}

#[test]
fn test_lower_coverage_boost() {
    // 1. Lowering types
    assert_eq!(lower_type(&rust_subset::RustType::Bool).unwrap(), WirType::Bool);
    assert_eq!(lower_type(&rust_subset::RustType::U8).unwrap(), WirType::U8);
    assert_eq!(lower_type(&rust_subset::RustType::U16).unwrap(), WirType::U16);
    assert_eq!(lower_type(&rust_subset::RustType::U32).unwrap(), WirType::U32);
    assert_eq!(lower_type(&rust_subset::RustType::U64).unwrap(), WirType::U64);
    assert_eq!(lower_type(&rust_subset::RustType::I8).unwrap(), WirType::I8);
    assert_eq!(lower_type(&rust_subset::RustType::I16).unwrap(), WirType::I16);
    assert_eq!(lower_type(&rust_subset::RustType::I32).unwrap(), WirType::I32);
    assert_eq!(lower_type(&rust_subset::RustType::I64).unwrap(), WirType::I64);
    assert_eq!(lower_type(&rust_subset::RustType::Unit).unwrap(), WirType::Unit);
    assert_eq!(lower_type(&rust_subset::RustType::Slice(Box::new(rust_subset::RustType::U8))).unwrap(), WirType::Ptr(Box::new(WirType::U8)));
    assert_eq!(lower_type(&rust_subset::RustType::Array(Box::new(rust_subset::RustType::U8), 4)).unwrap(), WirType::Ptr(Box::new(WirType::U8)));
    assert_eq!(lower_type(&rust_subset::RustType::ConstArray(Box::new(rust_subset::RustType::U8), "N".to_string())).unwrap(), WirType::Ptr(Box::new(WirType::U8)));
    assert_eq!(lower_type(&rust_subset::RustType::Vec(Box::new(rust_subset::RustType::U8))).unwrap(), WirType::Ptr(Box::new(WirType::U8)));
    assert!(lower_type(&rust_subset::RustType::Custom("Unknown".to_string())).is_err());

    // 2. Lowering all operators and expressions
    let snippet = r#"
        fn all_ops(a: i32, b: i32, flag: bool, ptr: &u8) -> i32 {
            let mut x = a + b - a * b / a % b;
            let sh = (a << 1) >> 1;
            let cmp = a < b && a <= b && a > b && a >= b;
            let logic = flag || !flag;
            let neg = -a;
            let deref = *ptr;
            x += 1;
            x -= 1;
            x |= 2;
            x &= 3;
            x ^= 4;
            x <<= 1;
            x >>= 1;
            return x;
        }
    "#;
    let file = parse_rust_file(snippet).unwrap();
    let prog = lower_source_file(&file, &[]).unwrap();
    assert!(prog.find_function("all_ops").is_some());

    // 3. Lowering Context methods
    let mut ctx = LoweringContext::default();
    let v1 = ctx.fresh_var("test");
    let b1 = ctx.fresh_block_id();
    ctx.push_scope();
    ctx.insert_var("x".to_string(), v1.clone(), WirType::I32, Label::PUBLIC);
    assert!(ctx.lookup_var("x").is_some());
    assert!(ctx.lookup_var("y").is_none());
    ctx.pop_scope();
    ctx.finish_block(b1, WirTerminator::Return(None));
}

#[test]
fn test_lower_exhaustive_coverage() {
    let snippet = r#"
        mod outer {
            use core::hint::black_box;
            pub fn test_comprehensive(
                p_u8: &u8,
                p_u16: &u16,
                p_u32: &u32,
                p_u64: &u64,
                s_i8: i8,
                u_u8: u8,
                u_u16: u16,
                u_u32: u32,
                u_u64: u64,
                flag: bool
            ) {
                // Literals and strings
                let l_u8 = 10u8;
                let l_u16 = 20u16;
                let l_u32 = 30u32;
                let l_u64 = 40u64;
                let l_us = 50usize;
                let l_i8 = 10i8;
                let l_i16 = 20i16;
                let l_i32 = 30i32;
                let l_i64 = 40i64;
                let l_is = 50isize;
                let bs = b"raw_bytes";
                let st = "string_slice";
                let max_val = u64::MAX;

                // Bitwise NOT across all integer widths
                let not_8 = !u_u8;
                let not_16 = !u_u16;
                let not_32 = !u_u32;
                let not_64 = !u_u64;

                // Casts: Truncation, SExt, ZExt, Identity
                let trunc_val = l_u64 as u8;
                let sext_val = s_i8 as i64;
                let zext_val = u_u8 as u64;
                let id_val = l_u32 as u32;

                // Methods and volatile primitives
                let wrap_sub = u_u64.wrapping_sub(1);
                let wrap_neg = u_u64.wrapping_neg();
                let vol_r = core::ptr::read_volatile(p_u8);
                core::ptr::write_volatile(p_u8, vol_r);

                // Macros
                assert!(flag);
                assert_eq!(l_u8, 10u8);
                debug_assert_eq!(l_u16, 20u16);

                // Arrays
                let arr_rep = [0u8; 8];
                let arr_lit = [1, 2, 3, 4];

                // If without else
                if flag {
                    let inside_if = 1;
                }

                // Range loop inclusive
                for idx in 0..=3 {
                    let k = idx;
                }

                // Unsafe block
                unsafe {
                    let un = 42;
                }

                return;
            }
        }
    "#;

    let file = parse_rust_file(snippet).expect("Failed to parse comprehensive snippet");
    let prog = lower_source_file(&file, &[]).expect("Failed to lower comprehensive snippet");
    assert!(prog.find_function("test_comprehensive").is_some());

    // Test error cases
    let mut err_ctx = LoweringContext::new();

    // 1. Variable not found
    let err_path = rust_subset::ast::Expr::Path(vec!["non_existent_var".to_string()], rust_subset::ast::Span::default());
    assert!(lower_expr(&mut err_ctx, &err_path).is_err());

    // 2. Unsupported integer suffix
    let err_lit = rust_subset::ast::Expr::Lit(
        rust_subset::ast::Lit::Int(42, Some("f32".to_string())),
        rust_subset::ast::Span::default(),
    );
    assert!(lower_expr(&mut err_ctx, &err_lit).is_err());

    // 3. wrapping_sub bad arg count
    let err_sub = rust_subset::ast::Expr::MethodCall(
        Box::new(rust_subset::ast::Expr::Lit(rust_subset::ast::Lit::Int(1, None), rust_subset::ast::Span::default())),
        "wrapping_sub".to_string(),
        vec![],
        rust_subset::ast::Span::default(),
    );
    assert!(lower_expr(&mut err_ctx, &err_sub).is_err());

    // 4. Unsupported method call
    let err_meth = rust_subset::ast::Expr::MethodCall(
        Box::new(rust_subset::ast::Expr::Lit(rust_subset::ast::Lit::Int(1, None), rust_subset::ast::Span::default())),
        "unknown_method".to_string(),
        vec![],
        rust_subset::ast::Span::default(),
    );
    assert!(lower_expr(&mut err_ctx, &err_meth).is_err());

    // 5. Unsupported macro
    let err_macro = rust_subset::ast::Expr::MacroCall(
        "unsupported_macro".to_string(),
        vec![],
        rust_subset::ast::Span::default(),
    );
    assert!(lower_expr(&mut err_ctx, &err_macro).is_err());

    // 6. Unsupported call
    let err_call = rust_subset::ast::Expr::Call(
        Box::new(rust_subset::ast::Expr::Lit(rust_subset::ast::Lit::Bool(true), rust_subset::ast::Span::default())),
        vec![],
        rust_subset::ast::Span::default(),
    );
    assert!(lower_expr(&mut err_ctx, &err_call).is_err());

    // 7. Unsupported expr variant
    let err_expr = rust_subset::ast::Expr::Range(None, None, false, rust_subset::ast::Span::default());
    assert!(lower_expr(&mut err_ctx, &err_expr).is_err());
}
