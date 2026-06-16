#[test]
fn n7_wrap_and_saturate() {
    use vm_core::core::VMCore;
    for (label, src, want) in [
        ("15u4 +% 1 (wrap)",   "fn main() -> u4 { return 15 +% 1; }", 0i64),
        ("200u8 +% 100 (wrap)","fn main() -> u8 { return 200 +% 100; }", 44),
        ("15u4 +? 1 (sat)",    "fn main() -> u4 { return 15 +? 1; }", 15),
        ("200u8 +? 100 (sat)", "fn main() -> u8 { return 200 +? 100; }", 255),
        ("3u8 +? 4 (no sat)",  "fn main() -> u8 { return 3 +? 4; }", 7),
    ] {
        let mut m = nib_iir_compiler::compile_source(src, "m").unwrap();
        let entry = m.entry_point.clone().unwrap();
        let mut vm = VMCore::new();
        let got = vm.execute(&mut m, &entry, &[]).unwrap().and_then(|v| v.as_i64()).unwrap();
        eprintln!("{label}: got {got} want {want}");
        assert_eq!(got, want, "{label}");
    }
}
