# jvm-class-file

`jvm-class-file` is the Rust class-file infrastructure layer for the repo's JVM
lane.

It intentionally models only the conservative subset we need first:

- parse a plain `.class` file with a constant pool, methods, and `Code`
  attributes
- resolve class, field, method, name-and-type, and loadable constants
- build a minimal one-method class file for tests and small tools

This crate is IR-agnostic. It does not know about Brainfuck, Nib, or the
compiler pipeline. Higher layers such as `ir-to-jvm-class-file` use it for
structural validation and low-level format understanding.

## Position in the stack

```text
source frontend
  -> compiler-ir
  -> ir-to-jvm-class-file
  -> jvm-class-file
  -> .class bytes
```

## Example

```rust
use jvm_class_file::{build_minimal_class_file, parse_class_file, BuildMinimalClassFileParams};

let bytes = build_minimal_class_file(BuildMinimalClassFileParams {
    class_name: "demo/Hello".to_string(),
    method_name: "main".to_string(),
    descriptor: "([Ljava/lang/String;)V".to_string(),
    code: vec![0xb1],
    max_stack: 0,
    max_locals: 1,
    ..Default::default()
})?;

let parsed = parse_class_file(&bytes)?;
assert_eq!(parsed.this_class_name, "demo/Hello");
# Ok::<(), Box<dyn std::error::Error>>(())
```
