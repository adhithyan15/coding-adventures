# build-tool: deps=c/iso-harness c/platform-harness c/os-platform c/net c/reactor c/tcp-runtime c/resp-protocol
# resp-server (OS-agnostic; rides tcp-runtime → net + reactor, speaks RESP via
# resp-protocol). run.sh compiles the server + test with all backends for this
# OS. Windows uses BUILD_windows (Winsock backends, linking ws2_32).
sh tools/run.sh
