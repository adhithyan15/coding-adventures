# ir-to-jvm-class-file

`ir-to-jvm-class-file` is the generic TypeScript JVM backend for the repo's
lower-level `compiler-ir` programs.

It accepts an `IrProgram`, emits conservative JVM bytecode inside a real
`.class` file, and writes the result using Java classpath layout when asked.

The emitted classes stay deliberately boring:

- static register and memory arrays
- one static JVM method per callable IR region
- plain integer arithmetic and array access
- no reflection, `invokedynamic`, proxies, or class loading tricks

## Writing class files safely

`writeClassFile(artifact, outputDir)` places `demo/Example.class` under
`outputDir`, creating package directories as needed, and refuses to write
through a symlink (or, on Windows, a junction) planted in the output tree. It
shells out to `python3` for the write, because Node has no `openat(2)`:

| Platform | How the tree is walked | Race-free against a concurrent swap? |
|----------|------------------------|--------------------------------------|
| POSIX    | `os.open(..., O_DIRECTORY \| O_NOFOLLOW, dir_fd=parent)` per component | yes |
| Windows  | `mkdir` + `lstat` per component, reject reparse points, write an `O_EXCL` temp file and `os.replace` it over the target | no — a directory swapped between check and write is not caught |

Both walks reject a root that is a file, a link anywhere at or below the
deepest existing root component, and an existing output that is not a plain
file.
