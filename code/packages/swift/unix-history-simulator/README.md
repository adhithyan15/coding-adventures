# Swift Unix History Simulator

A reusable, in-memory Swift port of the HOS00 First Edition Unix teaching
model. The library exposes filesystem, descriptor-table, and kernel
components. It has no command-line program and does not load repository
fixtures at runtime.

## Use

```swift
let filesystem = FileSystem.demo()
let descriptors = DescriptorTable()
let kernel = Kernel(filesystem: filesystem, descriptors: descriptors)
let trace = kernel.runCommand("cat /hello")
```

The trace contains output and ordered typed events. Its caller decides how to
display or serialize them. `DescriptorTable(capacity: -1)` throws `EINVAL`;
zero capacity is valid and cannot allocate descriptors.

The package BUILD recipe runs component checks and replays the nine shared
traces with an external test-only adapter. The adapter and fixture JSON are
outside the package API.
