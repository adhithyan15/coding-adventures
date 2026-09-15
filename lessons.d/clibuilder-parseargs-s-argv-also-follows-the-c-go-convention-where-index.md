---
category: Haskell
---

# `CliBuilder.parseArgs`'s `argv` also follows the C/Go convention where index 0 is the program name

(`parseArgs (Parser spec) args` pattern-matches `program : argv` and errors `"argv must have at least one element (the program name)"` on an empty list). `System.Environment.getArgs` does **not** include the program name — pass `"cowsay" : args`, not `args` directly. Same pitfall as the C# port (see `## C#` above); confirmed independently by reading `CliBuilder.hs`'s `parseArgs` rather than assuming from Perl's precedent (Perl's `CliBuilder` is the one exception in this repo — its `parse` iterates the whole array from index 0, no placeholder needed).
