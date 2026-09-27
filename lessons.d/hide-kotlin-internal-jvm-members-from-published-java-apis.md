# Hide Kotlin internal JVM members from published Java APIs

Kotlin `internal` is module visibility for Kotlin callers, not JVM bytecode
privacy: Java could invoke generated public constructors/getters, mutate an
element's backing bytes, and forge typed values. Published JVM libraries must
use private constructors plus synthetic module factories, return defensive
copies from synthetic helpers, and use runtime-unmodifiable collections for
properties exposed to Java.
