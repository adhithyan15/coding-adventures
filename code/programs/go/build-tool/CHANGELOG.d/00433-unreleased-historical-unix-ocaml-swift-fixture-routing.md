# Route historical Unix fixture changes to OCaml and Swift

The build-tool registry now selects the OCaml and Swift HOS00 packages for the
canonical nine-case fixture, shared replay helper, shared OCaml/Swift build
driver, and only each lane's fixture adapter source/project. The selector and
fallback package hash use the same exact inputs; adjacent paths and README
files remain excluded.
