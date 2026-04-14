#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

# Install the directed_graph dependency (transitive dep, leaf-to-root order).
# The directed_graph package must be installed before markov_chain can be
# built, because the rockspec declares it as a luarocks dependency.
(cd ../../lua/directed_graph && mise exec -- luarocks make --local *.rockspec)

# Install the markov_chain package itself.
mise exec -- luarocks make --local *.rockspec

# Run the test suite with busted.
# The tests/ directory contains test_markov_chain.lua which uses busted's
# describe/it/assert style.
cd tests && mise exec -- busted . --verbose --pattern=test_
