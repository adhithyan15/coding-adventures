// build.gradle.kts — Gradle build configuration for the Kotlin hello-world program.
//
// This file tells Gradle how to build and run the program. It uses the Kotlin
// DSL (hence the .kts extension), which means the build file itself is valid
// Kotlin code.
//
// PLUGINS
// -------
// The `kotlin("jvm")` plugin adds Kotlin compilation support targeting the
// JVM. The `application` plugin adds the ability to run the program via
// `gradle run` and package it as a distributable archive.
//
// KOTLIN vs JAVA GRADLE FILES
// ----------------------------
// This build file looks almost identical to the Java hello-world. The only
// difference is `kotlin("jvm")` instead of `java` in the plugins block.
// Gradle abstracts away the language-specific compilation steps — you just
// tell it which language plugin to use and it handles the rest.
//
// REPOSITORIES
// ------------
// `mavenCentral()` is the primary repository for JVM libraries (shared
// between Java and Kotlin, since both compile to JVM bytecode).

// GRADLE BUILD DIRECTORY
// ----------------------
// By default, Gradle outputs compiled classes, JARs, and reports to a
// directory called "build". This conflicts with our BUILD file on
// case-insensitive filesystems (macOS, Windows) where "BUILD" and "build"
// are the same name. We redirect Gradle's output to "gradle-build" to
// avoid this collision.
layout.buildDirectory = file("gradle-build")

plugins {
    kotlin("jvm") version "2.1.20"
    application
}

repositories {
    mavenCentral()
}

// The application plugin needs to know which class contains main().
// In Kotlin, a file-level fun main() in Main.kt gets compiled to a class
// named MainKt (the filename + "Kt" suffix). This is a Kotlin convention.
application {
    mainClass.set("com.codingadventures.helloworld.MainKt")
}

// JVM target version. We let Gradle use whatever JDK is available on the
// build machine. The Kotlin and Java compilation tasks will both default to
// the same JVM target (the running JDK's version), avoiding the
// "Inconsistent JVM-target compatibility" error that occurs when compileJava
// and compileKotlin target different versions.
