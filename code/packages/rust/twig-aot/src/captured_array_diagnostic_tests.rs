use super::*;

// Run the same preparation, inference, specialisation and backend entry used by
// the native object writer. A generic BackendRefused alone cannot identify the
// failure; the per-function backend diagnostic retains the actual ABI and arity.
#[test]
fn captured_array_exact_prepared_call_signature_diagnostic() {
    let sources = [
        include_str!("../tests/fixtures/captured-arrays/0.alg"),
        include_str!("../tests/fixtures/captured-arrays/1.alg"),
        include_str!("../tests/fixtures/captured-arrays/2.alg"),
        include_str!("../tests/fixtures/captured-arrays/3.alg"),
        include_str!("../tests/fixtures/captured-arrays/4.alg"),
        include_str!("../tests/fixtures/captured-arrays/5.alg"),
        include_str!("../tests/fixtures/captured-arrays/6.alg"),
        include_str!("../tests/fixtures/captured-arrays/7.alg"),
    ];
    for (case, source) in sources.iter().enumerate() {
        let mut module = algol_iir_compiler::compile_source(source, "array_diagnostic").unwrap();
        prepare_module_for_aot(&mut module);
        let globals = collect_global_slots(&module);
        for abi in [X86_64Abi::MsX64, X86_64Abi::SysV] {
            for function in &module.functions {
                let ctx = FunctionContext { name: &function.name, params: &function.params, return_type: &function.return_type };
                let inferred = infer_types(function);
                let cir = aot_specialise(function, Some(&inferred));
                let calls: Vec<_> = cir.iter().filter(|i| i.op == "call").map(|i| &i.srcs).collect();
                let result = x86_64_compile_with_stackmap(&ctx, &cir, abi, &globals);
                eprintln!("case={case} abi={abi:?} function={} params={:?} calls={calls:?} error={:?}", function.name, function.params, result.as_ref().err());
                assert!(result.is_ok(), "case={case} abi={abi:?} function={}: {:?}", function.name, result.err());
            }
        }
    }
}
