# nib-jvm-compiler

`nib-jvm-compiler` is the thin language-facing wrapper for lowering Go Nib
programs into JVM `.class` bytes.

The pipeline shape mirrors the Python JVM package and the repo's other Go
compiler orchestrators:

```text
Nib source
  -> nib-parser
  -> nib-type-checker
  -> nib-ir-compiler
  -> ir-optimizer
  -> ir-to-jvm-class-file
  -> jvm-class-file parser
  -> .class bytes
```

## Usage

```go
result, err := nibjvmcompiler.CompileSource("fn main() -> u4 { return 7; }")
if err != nil {
    panic(err)
}

fmt.Println(result.ClassName)
fmt.Println(result.ParsedClass.ThisClassName)
```
