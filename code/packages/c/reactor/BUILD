# build-tool: deps=c/iso-harness c/platform-harness c/os-platform
# reactor (Unix backends: macOS kqueue + Linux epoll — scalable O(ready)
# readiness). run.sh selects reactor_mac.c / reactor_linux.c by OS. Windows uses
# BUILD_windows (WSAPoll, linking ws2_32).
sh tools/run.sh
