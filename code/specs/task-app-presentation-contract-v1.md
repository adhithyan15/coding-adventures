# TaskApp web/native presentation contract v1

Issue: [#13521](https://github.com/adhithyan15/coding-adventures/issues/13521)

## Decision

TaskApp keeps two idiomatic presentation adapters: the React controller calls
`task-core` through `task-wasm`, while generated native hosts call it through
`task-mosaic-app`. They must not become two independently evolving products.
The data fixture at
`code/programs/mosaic/task-app/fixtures/presentation-contract-v1.json` is the
shared executable behavior contract for both adapters.

The fixture pins a UTC scheduling day and applies one ordered lifecycle:

1. initial state and List → Board navigation;
2. task name and due-date entry, creation, Full-CPM scheduling, completion,
   reopening, and return to List;
3. project creation/activation and switching back to the original project;
4. snapshot/restore; and
5. deletion.

Every checkpoint asserts the same canonical engine state and the same
user-visible core slots: selected view, summary, progress, complexity, composer
values, project rows, and the task row's completion/name/due/schedule cells.
The web test uses the production JavaScript ABI accessor plus a real release
`task-wasm` module; the Rust test exercises `TaskMosaicApp` directly. Updating
one adapter without updating the shared expectation therefore fails its own
package gate.

## Deliberate host-only state

- Web theme selection is stored in `localStorage`, because mosstyle emits a
  complete component per theme. Native hosts retain the portable theme flag in
  their Mosaic snapshot. Theme is excluded from this engine/presentation
  fixture rather than forcing either host into the other's storage model.
- Calendar month labels use host locale formatting. The fixture covers due-date
  scheduling but excludes locale-dependent calendar copy.

These exclusions are fixture metadata so adding another host-only difference
requires an explicit reviewable contract change.

## First-run repair

`task-core` intentionally creates a product-neutral root project with a blank
name. The native adapter already presents it as **Inbox**. The shared contract
exposed that web instead showed its internal id, `project`; the web controller
now repairs a blank initial or restored root name to **Inbox** before deriving
its first props.
