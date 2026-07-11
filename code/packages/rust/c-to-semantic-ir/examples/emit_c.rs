//! Lower a C snippet and print the C-backend re-emission (for 3-compiler checks).
fn main() {
    let src = std::env::args().nth(1).unwrap_or_else(|| {
        "int main(void) { uint8_t c = 200 + 100; printf(\"%d\n\", c); int32_t y = (int32_t)(2000000000 + 2000000000); printf(\"%d\n\", y); return 0; }".to_string()
    });
    let m = c_to_semantic_ir::compile_source(&src, "prog").expect("lower");
    print!("{}", semantic_ir_to_c::compile(&m).expect("emit c").source);
}
