# "No absolute paths" and "actually launches" are two different claims

Publishing the Qt app needed proof the bundle would work on someone else's
machine. `qt_add_executable` links the frameworks by absolute path
(`/opt/homebrew/opt/qtbase/lib/QtCore.framework/...`), so the binary runs
perfectly for whoever built it and fails for everyone else — the "runnable is
not working" trap one level down from a missing engine.

So I wrote a real check: parse the Mach-O load commands, collect every
`LC_LOAD_DYLIB` path, refuse anything not under `@executable_path`,
`@loader_path`, `@rpath`, or a system prefix. It was properly controlled — the
undeployed binary reported 8 absolute Homebrew paths, matching `otool` exactly;
`/bin/ls` and the SwiftUI app reported zero. After `macdeployqt` the bundle
reported **zero non-relocatable dependencies**.

Then I copied it somewhere else and ran it, and it exited instantly with no
output at all.

`macdeployqt` **invalidates every code signature it touches** when it rewrites
install names, and on arm64 the loader refuses a dylib whose signature does not
verify. `codesign --force --deep --sign -` fixed it. The dependency check was
correct and complete and told me nothing about this, because it answers a
different question.

What I'd otherwise have shipped: a payload that passed a thorough, well-
controlled, mutation-tested check and did not start.

- **A check answers the question it asks.** Mine asked "would every dependency
  resolve"; the user's question is "does it launch". Those coincide often
  enough to be mistaken for each other, and the only way to tell is to run the
  artifact — moved, extracted, from where a user would have it.
- **Two failure modes in the same step need two assertions.** Relocatability
  and signature validity are both properties of "deployed by macdeployqt", and
  one passing says nothing about the other. They are checked separately now.
- **Run it from where the user gets it.** Every payload this session was
  launched from an extraction of the actual published zip, in a different
  directory. For Qt that is what caught this; for the others it is what made
  "it works" mean anything.

Also worth recording: the Qt payload preserved **186 symlinks**, and the earlier
Compose macOS payload had zero. The symlink-storing work looked, at the time,
like a fix for a hypothetical — the bundle that motivated it turned out not to
contain any. Three payloads later it was load-bearing.
