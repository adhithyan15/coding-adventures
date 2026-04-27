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
