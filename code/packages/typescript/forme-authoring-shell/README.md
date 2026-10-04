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
identity factory. The shell snapshots callable handles and display records at
admission, rejects malformed workspaces, and never exposes those host handles
to the editor.

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
- Preview refresh is explicit and always uses the exact active session.
- Publication shows the complete reviewed target and destination, then
  requires a separate confirmation action.
- Host rejections and malformed values become fixed user-facing failures;
  adapter details never reach the DOM.
- Loading and creation are abortable. Preview and publisher handles share one
  idempotent retirement boundary, including late completion after unmount.

## Verification

The test suite covers first run, editor composition, preview and publication
outcomes, complete target review, retry, hostile admission data, loopback URL
restrictions, late async settlement, and shared disposal. Coverage exceeds 95%
statements and lines and 90% branches.
