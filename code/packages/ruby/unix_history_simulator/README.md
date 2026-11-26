# Unix History Simulator (Ruby)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library exposes
`UnixHistory::FileSystem`, `DescriptorTable`, `Kernel` and
`UnixHistory.run_command` independently.

With Ruby 3.2 or newer, run `ruby exe/unix-history cat /hello` to print console
output and ordered kernel events. Run `ruby test/test_simulator.rb` to replay
the shared fixtures and check components.
