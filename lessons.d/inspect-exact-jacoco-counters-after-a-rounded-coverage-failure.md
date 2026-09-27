# Inspect exact JaCoCo counters after a rounded coverage failure

Gradle rounded a 210-of-223 line result to `0.94`, hiding that exactly two more
covered lines were needed for the 95 percent threshold. Read JaCoCo XML after a
failure, calculate the exact deficit, and target a feasible uncovered path
instead of adding broad tests for expressions whose Kotlin line mappings do not
receive coverage credit.
