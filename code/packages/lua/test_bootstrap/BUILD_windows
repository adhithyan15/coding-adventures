# Runs every tests/test_*.lua: the unit suites, the shared pbt-v1 corpus (25/25) and
# the shared fuzz-v1 corpus (25/25) with its negative controls, and Go's 820 fuzz
# reference vectors (parsed from the committed Go table, not copied).
cd tests && busted . --verbose --pattern=test_
