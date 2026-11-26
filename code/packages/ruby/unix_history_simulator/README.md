# Unix History Simulator (Ruby)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library exposes
`UnixHistory::FileSystem`, `DescriptorTable`, `Kernel` and
`UnixHistory.run_command` independently.

`FileSystem#read` slices the UTF-8 byte representation of file data. Its offset
and count must be non-negative integers; invalid ranges return `EINVAL`.

With Ruby 3.2 or newer, run `ruby exe/unix-history cat /hello` to print console
output and ordered kernel events. Run `ruby test/test_simulator.rb` to replay
the shared fixtures and check components.
