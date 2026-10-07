---
category: Cross-platform & Windows BUILD_windows
---

# Code that only runs on phones still runs in a harness on Windows, where dart:io joins paths with a backslash

The Flutter phone file path (UI89 §7.11) built a request directory with
`Directory.createTemp`. It then checked the plugin's answer against
`'$directory/$name'` and staged saves at `'$dir/$name'`. Phones are POSIX,
so this looked safe. But the headless conformance harness runs that same
code on Linux, macOS **and Windows**, and on Windows `createTemp` names the
directory with `\`. The exact-string check could never match, so the
Windows lane failed on the first phone check.

Fix: join with `Platform.pathSeparator` in the code and in the harness's
fakes, and strip a trailing separator of either kind. Put the link-creating
cases behind `!Platform.isWindows`, as the desktop link checks already
were, because Windows needs a privilege for a symbolic link.

Do differently: before pushing Dart that builds or compares paths, read it
for every platform the *harness* runs on, not only the platform the code is
for. Look for `'$dir/` string joins and `startsWith('.../')` checks, and
check each `Link(...).createSync` is behind the harness's existing Windows
guard.
