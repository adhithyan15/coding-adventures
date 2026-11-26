---
category: Cross-platform & Windows BUILD_windows
---

# Long Windows PATH values can break child-shell command lookup

Local shell-based checks could not find `ping`, Ruby wrappers, or npm's `tsc`
shim even though their files and PATH entries existed. The inherited PATH was
12,475 characters. Adding tool directories to that same oversized value did not
help; PATHEXT was already present. A controlled cmd.exe probe found PING with
a short PATH but failed with a 16,120-character PATH containing the same prefix.

Microsoft documents that cmd.exe ignores inherited environment variables beyond
its [8,191-character limit](https://learn.microsoft.com/en-us/troubleshoot/windows-client/shell-experience/command-line-string-limitation).
Use a short child PATH containing only the selected SDK directories and Windows
system directories rather than appending the entire old PATH. With that scoped
environment, normal npm build/test scripts and both previously failing Go
executor timeout tests passed. No global user settings or test logic changed.
