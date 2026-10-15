# CV04a: canonical temporary paths in the macOS bootstrap fixture

Spec committed before test fixture changes. Actual hosted8d73f88f job116532526128
fails the portable success test: production bootstrap resolves output to
/private/var/... while the test expects /var/... . Raw log SHA256
7bf5f038b375d4b4ac0183d55cf0b504b38bb9a8b0c175b0b95d9e236f157f7e.

The fixture's root must use the same canonical path identity as production.
First add a local lexical-parent-path reproducer to the existing mock fixture
and demonstrate failure. Then resolve the fixture root when acquired. Keep exact
published environment bytes, config preservation, all16 mocked subprocesses,
five offline Cabal routes, per-stage refusals and effective driver assertions.
Canonical roots also ensure child config assertions execute on macOS rather
than accidentally skipping because parent path spelling differs.

No production helper/workflow/probe change, no optional gate or suppressed test.
Run the complete portable suite and Ruff, verify production helper/workflow
byte-exact319dc1fe and refresh independent review. Actual hosted macOS Python3.10
and real C/GHC/hsc2hs/Cabal linking remain required after publication. Local
mock success is not macOS link acceptance. Miri DEFERRED/PENDING; full goal OPEN.
