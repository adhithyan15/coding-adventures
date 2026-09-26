# Use Gradle project-dir when validation starts at repository root

A combined repository-root search followed by `gradle test` left Gradle in a
directory without a settings file. When validation begins at the repository
root, pass `-p` with the exact package directory (or use a separate command
whose working directory is the package) before invoking Gradle tasks.
