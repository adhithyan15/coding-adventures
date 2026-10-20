# Retry resource-sensitive build-guard coverage failures in isolation before changing unrelated code

The final full coverage run timed out in the unrelated build-guard fixture that
spawns a deliberately failing prerequisite install. The same test passed in the
ordinary full suite, and the documentation-only change did not touch the build
guard. Before changing production or test timeouts, rerun the failing file by
itself under coverage to distinguish a resource-contention flake from a real
regression, then rerun the required aggregate gate from a clean idle process.
