# Unix History Simulator (Go)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library exposes a
read-only filesystem, a descriptor table, and kernel calls. The command
adapter prints the same JSON trace as the other implementations.

Run `go run ./cmd/unix-history cat /hello` and `go test ./...` from here.
