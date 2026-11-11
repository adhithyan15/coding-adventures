---
category: Testing & coverage
---

# Fixture-specific server policy helpers must not be reused across certificate hostnames

The integration suite's `server_policy()` helper hard-coded the Let's Encrypt
fixture hostname. Reusing it while extending a real Sectigo chain failed at
server identity validation before the new ordered constraint policy could run.
Parameterize fixture-policy helpers by the expected reference identity, keep a
small wrapper for the common fixture, and inspect the checked-in certificate's
SANs before selecting the test hostname.
