---
category: Testing & coverage
---

# Execute generated driver argument vectors with portable quoting

The first real generated-driver test on Windows showed os.execv splitting arguments containing spaces, including synthetic macOS tool paths. Mocked process results had not exposed it. Preserve the d646 eight-pass/one-fail receipt and execute an explicit subprocess argument vector; relay normal child exit codes and re-raise terminating POSIX signals. The shell launcher remains limited to safely quoted executable paths and quoted argument forwarding. This is portable driver evidence, not a genuine macOS Haskell link receipt.
