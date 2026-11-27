# Unix History Simulator (Python)

The [HOS00](../../../specs/HOS00-historical-os-simulator.md) library exposes
`FileSystem`, `DescriptorTable`, `Kernel` and `run_command` independently.

From this package directory, run a command and print its trace:

```sh
PYTHONPATH=src python3 -m unix_history cat /hello
```

Run `bash BUILD` to replay the shared fixtures and check the components.
