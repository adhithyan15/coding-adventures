### Fixed — the Compose effect test picks serialization jars kotlinc can read

`the_emitted_compose_host_answers_effects` took the newest cached
`kotlinx-serialization` jars. A Gradle cache shared with a newer Compose
project holds 1.11.0, built by Kotlin 2.3, which a local kotlinc 2.1 refuses
("module was compiled with an incompatible version of Kotlin"), so the test
failed on a correct host. It now tries each cached json/core pair newest
first and steps down only on that metadata refusal; any other compile error
still fails. No version table to keep in step.
