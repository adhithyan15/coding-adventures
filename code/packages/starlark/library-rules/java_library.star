# ============================================================================
# java_library.star — Build rule for Java library packages
# ============================================================================
#
# Java packages in this monorepo use Gradle with the Kotlin DSL as their
# build system. Each package has its own build.gradle.kts that declares
# plugins, dependencies, and test configuration. Gradle handles compilation,
# dependency resolution, and test execution natively.
#
# HOW GRADLE WORKS IN THIS MONOREPO
# ----------------------------------
# Each Java package is a standalone Gradle project with its own
# build.gradle.kts and settings.gradle.kts. Dependencies between packages
# in the monorepo use Gradle's "composite builds" feature — the
# settings.gradle.kts file declares sibling projects via includeBuild():
#
#   includeBuild("../logic-gates")
#   includeBuild("../transistors")
#
# The java_library rule doesn't manage these — Gradle handles it. What the
# rule DOES manage is telling the build tool about these relationships so
# it can:
#   1. Build dependencies before dependents (topological order)
#   2. Propagate changes (if logic-gates changes, rebuild dependents)
#   3. Run independent packages in parallel
#
# BUILD AND TEST TOOLING
# ----------------------
# - Build: Gradle compiles Java sources and produces a JAR
# - Test: JUnit 5 via the Gradle test task
# - Coverage: Kover (JetBrains' coverage tool, works for Java and Kotlin)
# - Lint: Gradle's built-in Java compilation warnings
#
# EXAMPLE BUILD FILE
# ------------------
#   load("code/packages/starlark/library-rules/java_library.star", "java_library")
#
#   _targets = [
#       java_library(
#           name = "logic-gates",
#           srcs = ["src/**/*.java"],
#           deps = ["java/transistors"],
#       ),
#   ]
#
# ============================================================================

def java_library(name, srcs = [], deps = []):
    # Register a Java library target for the build system.
    #
    # The build tool will run these commands for a java_library target:
    #     gradle build    — compile sources and run tests
    #     gradle test     — run JUnit 5 tests (also run by build, but explicit for clarity)
    #
    # Args:
    #     name: The package name, matching the directory under
    #           code/packages/java/. For example, "logic-gates" corresponds
    #           to code/packages/java/logic-gates/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           For Java packages, this is typically ["src/**/*.java"] to
    #           track all Java source files, or include "build.gradle.kts"
    #           to rebuild when build config changes.
    #
    #           If empty, the build tool tracks all files in the package
    #           directory.
    #
    #     deps: Dependencies as "language/package-name" strings.
    #           Examples:
    #               ["java/transistors"]
    #               ["java/logic-gates", "java/arithmetic"]
    #
    #           These must mirror the includeBuild() entries in
    #           settings.gradle.kts.
    return {
        "rule": "java_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "gradle", "args": ["build"]},
            {"type": "cmd", "program": "gradle", "args": ["test"]},
        ],
    }
