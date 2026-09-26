# Treat all-lane portable coverage as an end-state gate during serial rollout

The aggregate DER ASN.1 coverage test intentionally requires every registered
lane and failed on still-incomplete C# placeholder files while the Kotlin lane
itself was complete. During a serial all-lane rollout, run fixture-schema and
package-native gates for each finished lane, record the aggregate gate's exact
remaining lane, and require the aggregate only after the final lane lands.
