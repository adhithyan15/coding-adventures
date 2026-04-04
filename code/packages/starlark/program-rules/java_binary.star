# ============================================================================
# java_binary.star — Build rule for Java executable programs
# ============================================================================
#
# Java binaries are programs with a main class containing a
# public static void main(String[] args) method. When you run
# "gradle run", Gradle compiles the source and executes the main class.
#
# JAVA BINARY vs JAVA LIBRARY
# ----------------------------
# The distinction in Gradle:
#   - Library: uses the `java-library` plugin (produces a JAR for others)
#   - Binary:  uses the `application` plugin (produces a runnable program)
#
# The application plugin adds a `run` task and can produce distribution
# archives (zip/tar) with start scripts for all platforms.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("code/packages/starlark/program-rules/java_binary.star", "java_binary")
#
#   _targets = [
#       java_binary(
#           name = "hello-world",
#           srcs = ["src/**/*.java"],
#           deps = [],
#       ),
#   ]
#
# ============================================================================

def java_binary(name, srcs = [], deps = []):
    # Register a Java binary (executable program) target.
    #
    # Gradle compiles the source and makes it runnable via `gradle run`.
    # The build tool runs:
    #     gradle build — compile and run tests
    #     gradle test  — run JUnit 5 tests (if any)
    #
    # Args:
    #     name: The program name, matching the directory under
    #           code/programs/java/. For example, "hello-world" maps to
    #           code/programs/java/hello-world/.
    #
    #     srcs: File paths or glob patterns for change detection.
    #           Typical: ["src/**/*.java", "build.gradle.kts"]
    #
    #     deps: Dependencies as "language/package-name" strings.
    return {
        "rule": "java_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "gradle", "args": ["build"]},
            {"type": "cmd", "program": "gradle", "args": ["test"]},
        ],
    }
