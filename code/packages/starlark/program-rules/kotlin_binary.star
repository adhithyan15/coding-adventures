# ============================================================================
# kotlin_binary.star — Build rule for Kotlin executable programs
# ============================================================================
#
# Kotlin binaries are programs with a top-level fun main() function (or a
# class with a companion main). Gradle compiles Kotlin to JVM bytecode and
# the application plugin makes it runnable via `gradle run`.
#
# KOTLIN BINARY vs KOTLIN LIBRARY
# --------------------------------
# The distinction in Gradle:
#   - Library: uses the `kotlin("jvm")` plugin (produces a JAR for others)
#   - Binary:  uses `kotlin("jvm")` + `application` plugin (runnable program)
#
# EXAMPLE BUILD FILE
# ------------------
#   load("code/packages/starlark/program-rules/kotlin_binary.star", "kotlin_binary")
#
#   _targets = [
#       kotlin_binary(
#           name = "hello-world",
#           srcs = ["src/**/*.kt"],
#           deps = [],
#       ),
#   ]
#
# ============================================================================

def kotlin_binary(name, srcs = [], deps = []):
    # Register a Kotlin binary (executable program) target.
    #
    # Gradle compiles Kotlin sources and makes the program runnable via
    # `gradle run`. The build tool runs:
    #     gradle build — compile and run tests
    #     gradle test  — run JUnit 5 tests (if any)
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/kotlin/. For example, "hello-world" maps to
    #           code/programs/kotlin/hello-world/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["src/**/*.kt", "build.gradle.kts"]
    #
    #     deps: Dependencies as "language/package-name" strings.
    return {
        "rule": "kotlin_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "gradle", "args": ["build"]},
            {"type": "cmd", "program": "gradle", "args": ["test"]},
        ],
    }
