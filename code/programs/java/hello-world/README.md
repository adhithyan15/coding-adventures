# hello-world (Java)

The classic Hello, World! program implemented in Java. This is the starting point for tracing a simple program through the entire computing stack — from source code down through the JVM, bytecode, CPU, and logic gates.

## How it works

Java compiles to JVM bytecode (not native machine code like Go or Rust). The JVM then executes the bytecode, optionally JIT-compiling hot paths to native instructions at runtime.

## Running

```bash
gradle run
```

## Build system

This program uses Gradle with the Kotlin DSL (`build.gradle.kts`). The `application` plugin provides the `run` task that compiles and executes the program.
