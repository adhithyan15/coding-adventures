## 0.180.0 — 2026-07-06 — BA string INPUT: managed columns (JVM + CLR)

The `INPUT A$` matrix cell (`10 INPUT A$ / 20 PRINT A$ / 30 END`, stdin `"OK"` →
`OK`) gains its two **managed** columns, so it now runs on `[Jvm, Clr, Vm, Jit]`.
Neither needed a new value model — `str` is already a host string object on both —
only a read-a-line host primitive returning that native string, mirroring numeric
`input_i64`'s `readLong` / `ReadLine`+`Int32.Parse`.

- **JVM** (`iir-to-jvm-class-file` 0.29.0): the validator now accepts a `str`
  result on `call_builtin`/`mov`; `input_str` lowers to `invokestatic
  env/BasicRuntime.readLine()Ljava/lang/String;` + `astore`. The harness
  `BASIC_RUNTIME_JAVA` gains a `readLine()` method (reads to newline/EOF, returns
  the line). Run-verified locally via real `javac`/`java`.
- **CLR** (`iir-to-cil-bytecode` 0.38.0): `input_str` lowers to `call string
  System.Console::ReadLine()` (no `Int32.Parse`), stored into the `System.String`
  local. Verified on the CLR column via real `ilasm`/`dotnet` in CI.

Wiring the static columns (native/LLVM `__twig_input_str` returning a `[i64
len][bytes]` heap handle; WASM `env.__input_str` writing into linear memory) is
the next slice of this arc.

