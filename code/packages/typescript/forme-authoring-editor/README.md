# `@coding-adventures/forme-authoring-editor`

The accessible default editor for Forme's durable authoring project. It renders
native labelled controls for site settings, document lifecycle, persistent
undo/redo, and the default Content IR blocks while routing every change through
an injected `AuthoringSession`.

The package owns no storage, filesystem, network, preview, publish, or plugin
runtime capability. A successful UI action means the authoring core accepted
and persisted one semantic command.

## Basic use

```tsx
import {
  AUTHORING_EDITOR_CSS,
  AuthoringEditor,
} from "@coding-adventures/forme-authoring-editor";

<AuthoringEditor
  session={session}
  themeOptions={[
    { id: "forme-classless", label: "Forme Classless" },
    { id: "high-contrast", label: "High contrast" },
  ]}
  createDocumentIdentity={() => hostUuidV7()}
/>;
```

`themeOptions` is a reviewed identifier/label list rather than raw CSS.
`createDocumentIdentity` receives no project or host handles. The component
also injects `AUTHORING_EDITOR_CSS`; hosts may import the exported string when
they need to place it in their own stylesheet pipeline.

## Default blocks

The editor creates and edits paragraph, heading, ordered or unordered list,
image, code block, blockquote, table, and link blocks. Insert, edit, reorder,
and remove each construct a complete immutable `replace-document-body`
command. Existing authorable block types outside that default set remain
movable and removable without being rewritten.

List and table text is preflighted before expansion. The editor caps rows,
columns, and total padded table cells below the core's node budget, preventing
ragged input from allocating an oversized intermediate tree before core
validation.

All operations use native form controls and buttons. Reordering has explicit
Move up and Move down controls, focus follows document and block operations,
and polite status plus assertive failure regions announce persistence results.
No workflow depends on dragging, hover, a pointer, or `contenteditable`.

## Declarative plugin slots

Plugins contribute bounded data, never React components or event handlers:

```ts
const pluginContributions = [{
  pluginId: "word-count",
  actionId: "summarize",
  slot: "document-toolbar",
  label: "Summarize document",
}];
```

The fixed slots are `site-toolbar`, `document-toolbar`, and `block-toolbar`.
Activation gives an injected `EditorPluginBridge` a frozen request containing
only the plugin/action identity, slot target, and a private validated project
snapshot. The bridge represents the separately sandboxed FM02 host and returns
exactly one `AuthoringCommand`, which the editor dispatches through the core.
Each button shows its validated plugin identity. Bridge calls receive an abort
signal, use a bounded deadline, and are cancelled when the editor unmounts; the
FM02 host must retire non-cooperative work after cancellation. Contribution
arrays reject prototypes, accessors, symbols, sparse or hidden data, duplicate
identities, control and bidi-format characters, unsafe identifiers, inspection
traps, and over-limit data. Project data is validated into a private
deep-frozen snapshot before it crosses the bridge.

## Development

```bash
bash BUILD
```

The gate installs the two local dependencies, compiles public declarations,
and runs browser-oriented component and hostile-input tests with greater than
95% statement/line and 90% branch coverage. See
[FM09](../../../specs/FM09-forme-authoring-shell.md) for the normative editor
and authority contract.
