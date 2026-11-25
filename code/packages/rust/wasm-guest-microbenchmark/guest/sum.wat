;; Inspectable source for the checked-in Rust module-construction code.
;; The binary is encoded from `WasmModule` in src/lib.rs so setup remains
;; deterministic and requires no external WAT compiler.
(module
  (func $sum_recursive (export "sum_recursive") (param $n i32) (result i32)
    (if (result i32)
      (i32.eqz (local.get $n))
      (then (i32.const 0))
      (else
        (i32.add
          (local.get $n)
          (call $sum_recursive (i32.sub (local.get $n) (i32.const 1)))))))
  (func $sum_loop (export "sum_loop") (param $n i32) (result i32)
    (local $remaining i32)
    (local $sum i32)
    (local.set $remaining (local.get $n))
    (local.set $sum (i32.const 0))
    (block $done
      (loop $again
        (br_if $done (i32.eqz (local.get $remaining)))
        (local.set $sum (i32.add (local.get $sum) (local.get $remaining)))
        (local.set $remaining (i32.sub (local.get $remaining) (i32.const 1)))
        (br $again)))
    (local.get $sum))
  (func $empty (export "empty") (result i32)
    (i32.const 0)))
