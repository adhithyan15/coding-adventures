# ir-to-jvm-class-file

`ir-to-jvm-class-file` lowers the repository's generic Go `compiler-ir`
programs into real JVM `.class` bytes.

It sits between the language frontends and the reusable `jvm-class-file`
package:

```text
Brainfuck / Nib frontend
  -> compiler-ir
  -> ir-to-jvm-class-file
  -> .class bytes
  -> java / GraalVM native-image
```

The backend deliberately emits a boring JVM subset:

- static integer register array
- static byte memory image
- one static JVM method per callable IR region
- integer arithmetic, array accesses, branches, and `invokestatic`

That keeps the output friendly to ordinary JVMs and to GraalVM Native Image.

## Usage

```go
program := compilerir.NewIrProgram("_start")
program.AddInstruction(compilerir.IrInstruction{
    Opcode:   compilerir.OpLabel,
    Operands: []compilerir.IrOperand{compilerir.IrLabel{Name: "_start"}},
    ID:       -1,
})
program.AddInstruction(compilerir.IrInstruction{
    Opcode:   compilerir.OpLoadImm,
    Operands: []compilerir.IrOperand{compilerir.IrRegister{Index: 1}, compilerir.IrImmediate{Value: 7}},
    ID:       0,
})
program.AddInstruction(compilerir.IrInstruction{
    Opcode: compilerir.OpHalt,
    ID:     1,
})

artifact, err := irtojvmclassfile.LowerIRToJvmClassFile(
    program,
    irtojvmclassfile.JvmBackendConfig{ClassName: "demo.Example"},
)
if err != nil {
    panic(err)
}

outputPath, err := irtojvmclassfile.WriteClassFile(artifact, "out")
if err != nil {
    panic(err)
}
fmt.Println(outputPath)
```

## Writing class files safely

`WriteClassFile(artifact, outputDir)` places `demo/Example.class` under
`outputDir`, creating package directories as needed, and refuses to write
through a symlink (or, on Windows, a junction) planted in the output tree:

| Platform | File | How the tree is walked | Race-free against a concurrent swap? |
|----------|------|------------------------|--------------------------------------|
| Unix     | `backend_unix.go` | `openat(parent, child, O_DIRECTORY\|O_NOFOLLOW)` per component | yes |
| Windows  | `backend_windows.go` + `backend_pathwalk.go` | `Mkdir` + `Lstat` per component, reject reparse points, write an `O_EXCL` temp file and `os.Rename` it over the target | no: a directory swapped for a junction between check and write is not caught |
| other    | `backend_other.go` | refuses | n/a |

Both walks reject a root that is a file or a link, a link in the package
directories, and an existing output that is not a plain file. Class filenames
whose components contain `\`, `:` or NUL are rejected everywhere. The tests run
every write-safety case under both walks, so POSIX CI covers the Windows code
path as well.
