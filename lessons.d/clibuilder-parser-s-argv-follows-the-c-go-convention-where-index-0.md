---
category: C#
---

# `CliBuilder.Parser`'s `argv` follows the C/Go convention where index 0 is the program name

(`Parser.Parse()` sets `program = argv[0]` and starts real token parsing at `index = 1`). C#'s top-level `args` array does **not** include the program name — passing it straight to `new Parser(specPath, args)` silently drops the first real CLI token (single-arg invocations parse zero positional arguments; multi-arg invocations lose the first one). Symptom is silent: no exception, just an empty/short result. Fix: prepend a placeholder before calling, `var argv = new List<string> { "<program-name>" }; argv.AddRange(args);`. Found while wiring the C# `cowsay` port to `cli-builder` (first C# consumer of `Parser` outside its own test suite) — verify with an end-to-end run (`dotnet run -- <realistic args>`), not just unit tests that call `Parser` with a hand-built `argv` list, since it's easy to hand-build the list "correctly" (with a leading program name) in a test and then get the real entry point wrong.
