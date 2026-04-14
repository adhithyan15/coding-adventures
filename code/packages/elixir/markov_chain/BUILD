#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
(cd ../../elixir/directed_graph && mise exec -- mix deps.get && mise exec -- mix compile)
mise exec -- mix deps.get
mise exec -- mix test --cover
