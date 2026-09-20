# Run repository unittest modules from their import root

Invoking these tests as `code.scripts.tests.*` makes Python resolve the standard
library `code` module, which is not a package and therefore cannot contain the
repository's `scripts` namespace. Run the unittest command from
`code/scripts/tests` and name the local `test_*` modules directly so their
sibling imports and repository path setup behave as designed.
