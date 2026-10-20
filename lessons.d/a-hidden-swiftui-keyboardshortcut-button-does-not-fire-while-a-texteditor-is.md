---
category: Mosaic compiler pipeline
---

# A hidden SwiftUI keyboardShortcut button does not fire while a TextEditor is first responder, but onKeyPress on the editor does

**What happened.** UI25 §4's text-area commit (Cmd+Return) needed a SwiftUI
lowering that works on both macOS and iOS. The usual recipe is a hidden
`Button` with `.keyboardShortcut(.return, modifiers: .command)` in the
editor's `.background`, gated on `@FocusState`. It compiles everywhere, and
it looks right in review. Probed in a real AppKit-hosted `TextEditor` (a
small `swiftc` program that makes the editor first responder and sends
synthesized `NSEvent` key presses through `NSApp.sendEvent`), the shortcut
never fired, gated or not. `.onKeyPress(keys: [.return, "\u{3}"], phases:
.down)` on the editor fired for Cmd+Return, Ctrl+Return and Cmd+keypad Enter,
committed only the focused editor of two, and returning `.ignored` for bare
Return left the newline intact.

**Fix.** Lower the commit to a generated `_MosaicCommitChord` view modifier
built on `onKeyPress`. It is macOS 14 / iOS 17, newer than the generated
package's floor (macOS 13 / iOS 16), so the helper wraps it in
`if #available(macOS 14.0, iOS 17.0, *)`, with the plain content otherwise.
Then drive the *generated* view with the same probe, not just a hand-written
copy of it.

**Do differently.** For any SwiftUI keyboard behaviour, run it before
choosing: a 60-line `swiftc main.swift` program with an `NSHostingView` and
`NSEvent.keyEvent(...)` answers in seconds, headless enough to run from the
shell. "Compiles on both platforms" says nothing about whether the key
reaches the handler while a text view owns first responder.
