# Forme Authoring Shell

`@coding-adventures/forme-authoring-shell` is the capability-free FM09 React
composition for Forme's first-run authoring product. It joins the durable
authoring session, accessible block editor, exact-revision preview coordinator,
and reviewed publication coordinator without receiving filesystem, process,
credential, or network authority.

## Host boundary

The host supplies a closed theme list plus two operations: open an existing
workspace or create one from a bounded title and reviewed theme identity. A
workspace contains an active-draft `AuthoringSession`, one preview coordinator,
one or more reviewed publishers, a loopback HTTP preview URL, and a document
identity factory. It also supplies one idempotent workspace-level `dispose()`
method that owns retirement of the entire host graph. The shell captures that
method first, snapshots callable handles and bounded display records at
admission, rejects malformed workspaces, and never exposes raw host sessions or
handles to the editor.

```tsx
import { AuthoringShell } from "@coding-adventures/forme-authoring-shell";

root.render(<AuthoringShell host={{
  themes: reviewedThemes,
  open: (_signal) => desktopHost.openWorkspace(),
  create: (input, _signal) => desktopHost.createWorkspace(input),
}} />);
```

The native FM-B068 host is responsible for contained crash-safe storage,
product pipeline materialization, preview serving, target configuration,
credential custody, and packaging. Those capabilities are intentionally absent
from this package; `required_capabilities.json` remains empty.

## Product behavior

- A missing workspace produces a bounded first-run title and theme form.
- An admitted workspace renders the established editor and a sandboxed
  loopback preview frame.
- Preview refresh is explicit, uses a validated facade for the exact active
  session, and accepts only an exact-revision bounded result.
- Publication shows the complete reviewed target and destination, then
  requires a separate confirmation action and accepts only an exact-revision,
  exact-target result with a valid manifest digest.
- Host rejections and malformed values become fixed user-facing failures;
  adapter details never reach the DOM.
- Loading and creation are abortable. A synchronous action token rejects
  overlapping create, preview, and publish actions before React state settles.
- Workspace-level disposal is shared by failed admission, replacement, and
  late completion after unmount; disposal failures remain fixed and redacted.

## Verification

The test suite covers first run, editor composition, preview and publication
outcomes, complete target review, retry, hostile admission data, loopback URL
restrictions, synchronous throws, hostile action results, overlapping actions,
stale async settlement, session snapshot isolation, and shared disposal.
Coverage exceeds 95% statements and lines and 90% branches.
