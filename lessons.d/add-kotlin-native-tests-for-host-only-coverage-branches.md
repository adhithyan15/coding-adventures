# Add Kotlin-native tests for host-only coverage branches

All 109 neutral cases passed, but JaCoCo still reported only 93 percent line
coverage because several Kotlin host branches and compact error expressions did
not receive line credit from the generic projector. Add focused native tests
for canonicality, tag, bit-padding, NULL, and OID branches, then rerun the real
95 percent coverage gate rather than treating fixture completeness as coverage.
