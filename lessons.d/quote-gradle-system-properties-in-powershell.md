# Quote Gradle system properties in PowerShell

PowerShell split an unquoted Gradle `-Dkotlin.compiler.execution.strategy=...`
argument so Gradle interpreted the tail as a task name. Quote the entire `-D`
argument when invoking Gradle from PowerShell; keep the unquoted spelling in
the shell BUILD recipe where POSIX argument parsing preserves it.
