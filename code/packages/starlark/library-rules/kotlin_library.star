# ============================================================================
# kotlin_library.star — Build rule for Kotlin library packages
# ============================================================================
#
# Kotlin packages in this monorepo use Gradle with the Kotlin DSL as their
# build system, identical to Java packages. The only difference is the
# source language — Kotlin compiles to the same JVM bytecode as Java, and
# Gradle handles both transparently.
#
# HOW GRADLE WORKS IN THIS MONOREPO
# ----------------------------------
# Each Kotlin package is a standalone Gradle project with its own
# build.gradle.kts and settings.gradle.kts. Dependencies between packages
# use Gradle's "composite builds" feature:
#
#   includeBuild("../logic-gates")
#   includeBuild("../transistors")
#
# BUILD AND TEST TOOLING
# ----------------------
# - Build: Gradle compiles Kotlin sources via the kotlin("jvm") plugin
# - Test: JUnit 5 via the Gradle test task
# - Coverage: Kover (JetBrains' official Kotlin coverage tool)
# - Lint: Kotlin compiler warnings + optional detekt/ktlint
#
# EXAMPLE BUILD FILE
# ------------------
#   load("code/packages/starlark/library-rules/kotlin_library.star", "kotlin_library")
#
#   _targets = [
#       kotlin_library(
#           name = "logic-gates",
#           srcs = ["src/**/*.kt"],
#           deps = ["kotlin/transistors"],
#       ),
#   ]
#
# ============================================================================

def kotlin_library(name, srcs = [], deps = []):
    # Register a Kotlin library target for the build system.
    #
    # The build tool will run these commands for a kotlin_library target:
    #     gradle build    — compile Kotlin sources and run tests
    #     gradle test     — run JUnit 5 tests
    #
    # Args:
    #     name: The package name, matching the directory under
    #           code/packages/kotlin/. For example, "logic-gates"
    #           corresponds to code/packages/kotlin/logic-gates/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           For Kotlin packages, this is typically ["src/**/*.kt"]
    #           to track all Kotlin source files.
    #
    #           If empty, the build tool tracks all files in the package
    #           directory.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["kotlin/transistors"]
    #               ["kotlin/logic-gates", "kotlin/arithmetic"]
    #
    #           These must mirror the includeBuild() entries in
    #           settings.gradle.kts.
    return {
        "rule": "kotlin_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "gradle", "args": ["build"]},
            {"type": "cmd", "program": "gradle", "args": ["test"]},
        ],
    }
