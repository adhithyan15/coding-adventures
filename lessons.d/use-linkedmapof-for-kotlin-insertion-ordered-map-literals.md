# Use linkedMapOf for Kotlin insertion-ordered map literals

Kotlin's standard insertion-ordered map factory is `linkedMapOf`; `linkedMap`
does not exist. Fixture-projection tests should use `linkedMapOf` when stable
field insertion order is useful, and compile the test sources before spending
time on a full coverage run.
