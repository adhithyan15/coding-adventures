## 0.226.26 - 2026-08-15 (CLR keeps i64 on the real-CoreCLR path)

`concretize_scalar_any_for_cil` narrowed every `i64` to `i32` before lowering.
That is right for **one** of the CLR's two back ends and wrong for the other,
and the function could not tell them apart, so it took a `narrow_i64` parameter:

* **binary artifact path** (`compile_source_to_cil_artifact`) — `true`. It
  targets the in-repo `clr-simulator`, a 32-bit integer machine where a genuine
  `i64` has nowhere to live.
* **textual `.il` path** (`compile_source_to_cil_text`) — `false`. It targets
  real CoreCLR via `ilasm`, which has a native `int64`, and
  `iir-to-cil-bytecode` 0.45.0 now gives the emitter a real int64 register
  model. Narrowing there threw away a width the target actually has.

The narrowing is what turned COBOL's inherent scale-12 intermediate (`10^12`)
into an `int32` literal the backend could only (correctly) refuse. Bare
`any`/`polymorphic`/`ref<any>` still narrow to `i32` on both paths — an
unresolved dynamic value carries no evidence that it needs 64 bits.

Matrix: the COBOL `COMPUTE R = A / B + C` cell (`PIC 9(4)V99` → `000533`) is now
green on Clr.

