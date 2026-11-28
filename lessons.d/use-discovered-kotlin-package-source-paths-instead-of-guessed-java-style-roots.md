---
category: Repo policy / workflow reminders
---

# Use discovered Kotlin package source paths instead of guessed Java-style roots

Kotlin package trees in this repository may include an additional language
segment such as `com/codingadventures/kotlin/...`, unlike Java's package path.
Before opening source or test files, run `rg --files <package>` and use the
reported path rather than translating a Java path by guesswork.
