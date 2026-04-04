// Main.java — The first Java program: Hello, World!
//
// This is the Java implementation of the hello-world program that traces the
// entire coding-adventures computing stack. The long-term goal is to follow
// this simple program all the way down through the layers:
//
//     Source code (this file)
//     → Lexer (tokenize Java source into keywords, identifiers, literals)
//     → Parser (build an Abstract Syntax Tree)
//     → Compiler (emit JVM bytecode — .class files)
//     → JVM (execute bytecode via the fetch-decode-execute cycle)
//     → JIT Compiler (translate hot bytecode to native machine instructions)
//     → CPU (execute native instructions)
//     → ALU (arithmetic operations)
//     → Logic Gates (AND, OR, NOT — the foundation)
//
// JAVA'S EXECUTION MODEL
// ----------------------
// Unlike Go (which compiles directly to native machine code), Java takes a
// two-step approach:
//
//   1. The Java compiler (javac) compiles .java files to .class files
//      containing JVM bytecode — a platform-independent intermediate format.
//
//   2. The JVM (Java Virtual Machine) executes the bytecode. Modern JVMs
//      include a JIT (Just-In-Time) compiler that translates frequently
//      executed bytecode into native machine code at runtime.
//
// This "compile once, run anywhere" model is why Java runs on everything
// from embedded devices to cloud servers — the JVM abstracts away the
// hardware differences.
//
// THE main() METHOD
// -----------------
// Every Java program starts at a main() method. It must be:
//   - public: accessible from outside the class (the JVM calls it)
//   - static: callable without creating an instance of the class
//   - void: doesn't return a value
//   - String[] args: command-line arguments (even if unused)
//
// This is more ceremony than Go's `func main()` or Kotlin's `fun main()`,
// but it makes the entry point completely explicit and unambiguous.

package com.codingadventures.helloworld;

public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
