#!/bin/sh
set -eu
cd "$(dirname "$0")/../example/consumer"
mix deps.get --quiet
mix run -e 'if MicrobenchmarkConsumer.run() != :ok, do: System.halt(1)'
