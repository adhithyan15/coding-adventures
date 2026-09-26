### Fixed -- a handler that closed the host could jump into a freed module

Found by the security review, and the sharpest consequence of this whole arc:
the effect handler is the **first application callback that runs while native
code is live**, and `Monitor` is reentrant, so a handler is free to call the
public `Close()` from inside the settle. This host is also the only one of the
five that **unmaps** the runtime -- `Dispose` calls `NativeLibrary.Free` -- after
which the settle loop would still drive `CompleteEffectOnce` and
`PersistSnapshot` through delegates holding raw addresses inside that module.
An `AccessViolationException` is uncatchable in .NET, so the good outcome is a
dead process and the bad one is a jump into whatever got mapped there next.

"The user shut the window while the import dialog was open" reaches it.

The unload now waits for the outermost settle frame; the loop stops driving
rounds once a handler has closed the host, the way Qt re-checks its `QPointer`
for the same reason; and the two native call sites refuse a null handle. None
of the other four hosts has the exposure -- Compose, Flutter and SwiftUI never
unmap, and Qt only unloads from its destructor.

**Not pinned by the acceptance, and worth being exact about why.** The new
`closes` case exercises the path and asserts the process survives, but it does
not discriminate the fix on macOS: `dlclose` there returns success while
leaving the module mapped, so the dangerous call still lands on live code. That
was measured, not assumed -- a C probe confirmed the symbol's first byte is
still readable after a successful `dlclose`. Windows `FreeLibrary` does unmap,
so the case should discriminate on the platform this host actually ships to.

