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
- A malformed or rejected publisher settlement is conservatively reported as
  indeterminate because the shell cannot prove whether an external commit
  occurred; only a validated coordinator result may claim a pre-commit failure.
- Host rejections and malformed values become fixed user-facing failures;
  adapter details never reach the DOM.
- Loading and creation are abortable. A synchronous action token rejects
  overlapping create, preview, and publish actions before React state settles.
- A session mutation is not considered safely reflected until the shell can
  resnapshot the committed project, history flags, and storage revision. If
  that resnapshot fails, the workspace is poisoned and all interaction is
  removed until the product reloads from durable state.
- Workspace replacement is serialized behind successful retirement of the
  previous workspace. Workspace-level disposal is also shared by failed
  admission and late completion; a missing, rejected, or stale retirement
  boundary poisons the shell, retires any active replacement, and requires a
  reload rather than allowing possibly overlapping host graphs.

## Verification

The test suite covers first run, editor composition, preview and publication
outcomes, complete target review, retry, hostile admission data, loopback URL
restrictions, synchronous throws, hostile action results, overlapping actions,
stale async settlement, session snapshot isolation, and shared disposal.
Coverage meets 95% statements, lines, and functions and 90% branches.
