# JVM Class File (Go)

This package is the reusable class-file seam for the Go JVM lane. It does two
small but important jobs:

- parse a conservative subset of JVM `.class` files
- build a tiny valid `.class` file directly from bytecode plus metadata

That keeps the next backend split honest:

```text
compiler-ir -> ir-to-jvm-class-file -> jvm-class-file -> .class
```

`jvm-class-file` knows about class-file structure. It does **not** know about
Brainfuck, Nib, or the repository IR.

## What The MVP Supports

- constant-pool parsing for UTF-8, integer, long, double, class, string,
  name-and-type, field refs, and method refs
- method parsing
- structured `Code` attribute parsing
- helper lookups for constants and references
- building one minimal class containing one method

## Usage

```go
package main

import (
	"fmt"

	jvmclassfile "github.com/adhithyan15/coding-adventures/code/packages/go/jvm-class-file"
)

func main() {
	classBytes, err := jvmclassfile.BuildMinimalClassFile(
		jvmclassfile.BuildMinimalClassFileParams{
			ClassName:  "Example",
			MethodName: "compute",
			Descriptor: "()I",
			Code:       []byte{0x04, 0x05, 0x60, 0xAC},
			MaxStack:   2,
			MaxLocals:  0,
			Constants:  []any{int32(300), "hello"},
		},
	)
	if err != nil {
		panic(err)
	}

	parsed, err := jvmclassfile.ParseClassFile(classBytes)
	if err != nil {
		panic(err)
	}

	fmt.Println(parsed.ThisClassName)
}
```

## Why It Exists

Python already has `jvm-class-file` and `ir-to-jvm-class-file`. For Go, the
next truthful port starts here so later packages can share one tested class-file
model instead of shelling out to Python.
