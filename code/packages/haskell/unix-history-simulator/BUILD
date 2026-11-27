# needs-toolchain: python
if [ "$(uname -s)" = Darwin ]; then export DEVELOPER_DIR=/Library/Developer/CommandLineTools; fi; cabal build unix-history-simulator && cabal test unix-history-simulator:simulator-test && cabal build historical-unix-fixture-cli:exe:fixture-cli && python3 ../../../specs/fixtures/historical-unix-v1/replay_cli.py cabal run historical-unix-fixture-cli:exe:fixture-cli --
