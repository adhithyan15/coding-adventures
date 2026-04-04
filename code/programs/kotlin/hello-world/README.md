# hello-world (Kotlin)

The classic Hello, World! program implemented in Kotlin. This is a plain JVM console program (not an Android app) that demonstrates Kotlin's concise syntax compared to Java.

## How it works

Kotlin compiles to the same JVM bytecode as Java. The Kotlin compiler transforms the concise `fun main()` syntax into a standard JVM entry point. At runtime, there is no difference between Kotlin and Java bytecode.

## Running

```bash
gradle run
```

## Build system

This program uses Gradle with the Kotlin DSL (`build.gradle.kts`). The `kotlin("jvm")` plugin provides Kotlin compilation targeting the JVM, and the `application` plugin provides the `run` task.
