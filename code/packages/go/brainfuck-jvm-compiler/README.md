# brainfuck-jvm-compiler

`brainfuck-jvm-compiler` is the thin language-facing wrapper for the Go JVM
lane.

It mirrors the existing WASM pipeline style:

```text
Brainfuck source
  -> brainfuck
  -> brainfuck-ir-compiler
  -> ir-optimizer
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> .class bytes
```

The package does not lower JVM bytecode itself. It orchestrates the frontend,
the generic optimizer, and the generic JVM backend, then immediately parses the
generated class back through `jvm-class-file` as a structural self-check.

## Usage

```go
result, err := brainfuckjvmcompiler.CompileSource("+.")
if err != nil {
    panic(err)
}

fmt.Println(result.ClassName)
fmt.Println(len(result.ClassBytes))
```
