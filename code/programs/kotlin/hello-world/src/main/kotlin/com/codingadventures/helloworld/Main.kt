// Main.kt — The first Kotlin program: Hello, World!
//
// This is the Kotlin implementation of the hello-world program. Kotlin is a
// modern language that runs on the JVM, designed to be more concise and safer
// than Java while remaining fully interoperable with it.
//
// KOTLIN'S EXECUTION MODEL
// ------------------------
// Kotlin follows the same two-step execution model as Java:
//
//     Source code (this file, .kt)
//     → Kotlin Compiler (kotlinc) → JVM bytecode (.class files)
//     → JVM → JIT Compiler → native machine instructions
//     → CPU → ALU → Logic Gates
//
// The key difference from Java is at the SOURCE level — Kotlin's syntax is
// more concise. At the bytecode level, Java and Kotlin are identical. The
// JVM can't tell whether bytecode came from a .java or .kt file.
//
// KOTLIN vs JAVA: main()
// ----------------------
// Compare this file to the Java version:
//
//   Java:   public class Main {
//               public static void main(String[] args) {
//                   System.out.println("Hello, World!");
//               }
//           }
//
//   Kotlin: fun main() {
//               println("Hello, World!")
//           }
//
// Kotlin removes the boilerplate:
//   - No class wrapper needed (top-level functions are allowed)
//   - No public/static/void ceremony (the compiler handles it)
//   - No String[] args required (optional in Kotlin)
//   - No semicolons
//   - println() instead of System.out.println()
//
// Under the hood, the Kotlin compiler generates a class named MainKt (from
// the filename Main.kt) with a static main() method — exactly what the JVM
// expects. The conciseness is a compile-time convenience, not a runtime
// difference.

package com.codingadventures.helloworld

fun main() {
    println("Hello, World!")
}
