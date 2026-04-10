// Hello, World! — F# on .NET 9
//
// This is the computing-stack journey starting point, implemented in F#.
//
// F# is a functional-first language on the .NET CLR. The compilation
// pipeline is identical to C# at the binary level:
//
//   Source code (this file)
//   → F# compiler (fsc) — parses F# syntax, performs type inference,
//                          emits CIL (Common Intermediate Language) bytecode
//   → CLR JIT compiler  — compiles CIL to native machine code at runtime
//   → CPU execution     — printfn ultimately calls write() on the OS kernel
//
// F#'s computation model is rooted in the lambda calculus. Functions are
// first-class values, and immutability is the default. This makes F# a
// natural companion to the other functional languages in this repo:
//
//   Haskell  — pure functional, lazy evaluation, Hindley-Milner types
//   Elixir   — functional, actor model, runs on the BEAM VM
//   F#       — functional-first, eager evaluation, runs on the CLR
//
// All three compile to intermediate bytecode and share the insight that
// programs are best expressed as transformations of immutable data.
//
// The [<EntryPoint>] attribute marks this function as the program's entry
// point. It must accept a string array (command-line arguments) and return
// an int (exit code). Returning 0 is the Unix convention for success.

[<EntryPoint>]
let main _ =
    printfn "Hello, World!"
    0
