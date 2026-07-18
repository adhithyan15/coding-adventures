# build-tool: deps=c/iso-harness c/platform-harness c/os-platform
# vault-revisions (OS-agnostic thread-safe revision store; rides os-platform
# thread = osp_mutex). run.sh compiles the store + test with the os-platform
# thread backend for this OS. Windows uses BUILD_windows (CRT + kernel32).
sh tools/run.sh
