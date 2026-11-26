# Unix History Simulator (C++)

[HOS00](../../../specs/HOS00-historical-os-simulator.md) kernel model with a
reusable C++17 filesystem, descriptor table and syscall dispatcher. Run
`bash BUILD` for component checks and shared fixture replay.

```sh
c++ -std=c++17 -Iinclude src/unix_history.cpp src/cli.cpp -o unix-history
./unix-history cat /hello
```
