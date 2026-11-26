# Unix History Simulator (C)

[HOS00](../../../specs/HOS00-historical-os-simulator.md) kernel model with a reusable
C interface in `include/unix_history.h`. The filesystem owns lookup and byte
ranges. The caller provides the descriptor storage and can reuse it on its own.

Run `bash BUILD` for component checks and shared replay. To build the CLI on Unix:

```sh
cc -std=c11 -Iinclude src/*.c -o unix-history
./unix-history "cat /hello"
```
