// Hello, World! — C# on .NET 9
//
// This is the computing-stack journey starting point, implemented in C#.
//
// What happens when you run this program:
//
//   Source code (this file)
//   → Roslyn compiler  — parses C# syntax, applies semantic analysis,
//                         emits CIL (Common Intermediate Language) bytecode
//                         into a .dll assembly
//   → CLR JIT compiler — reads CIL at runtime, compiles it to native
//                         machine code for the current CPU architecture
//                         (x64, ARM64, etc.) the first time each method runs
//   → CPU execution    — executes the native instructions; Console.WriteLine
//                         ultimately issues a write() syscall to the OS kernel
//
// This mirrors what we are building by hand in the coding-adventures stack:
//
//   our lexer       ≈  Roslyn tokeniser
//   our parser      ≈  Roslyn syntax tree builder
//   our compiler    ≈  Roslyn IL emitter
//   our VM          ≈  CLR execution engine
//   our CPU sim     ≈  the hardware the CLR runs on
//
// C# uses "top-level statements" (introduced in C# 9, .NET 5): the entry
// point of a console program can be written as bare statements without
// wrapping them in a class or a Main method. The compiler synthesises the
// boilerplate automatically.

Console.WriteLine("Hello, World!");
