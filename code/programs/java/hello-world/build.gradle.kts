// build.gradle.kts — Gradle build configuration for the Java hello-world program.
//
// This file tells Gradle how to build and run the program. It uses the Kotlin
// DSL (hence the .kts extension), which means the build file itself is valid
// Kotlin code. This is the standard way to configure Gradle in modern Java
// and Kotlin projects.
//
// PLUGINS
// -------
// Gradle plugins add capabilities to the build. The `java` plugin provides
// Java compilation, and the `application` plugin adds the ability to run
// the program via `gradle run` and package it as a distributable archive.
//
// REPOSITORIES
// ------------
// `mavenCentral()` tells Gradle where to download dependencies from. Maven
// Central is the primary repository for JVM libraries (like npm for
// JavaScript or PyPI for Python).
//
// DEPENDENCIES
// ------------
// This hello-world has no external dependencies. When you add a library
// package as a dependency later, it looks like:
//   implementation("com.codingadventures:logic-gates")
// For monorepo siblings, you use composite builds (see settings.gradle.kts).

plugins {
    java
    application
}

repositories {
    mavenCentral()
}

// The application plugin needs to know which class contains main().
// The mainClass property takes the fully-qualified class name.
application {
    mainClass.set("com.codingadventures.helloworld.Main")
}

// Java version compatibility. Java 21 is the current LTS release.
java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(21))
    }
}
