# TaskApp first-run usability audit v1

Issue: [#13523](https://github.com/adhithyan15/coding-adventures/issues/13523)

## Product test

Trestle must be useful as a plain, local todo list before a person learns that it
also contains a Rust scheduling engine. The default path is therefore:

1. launch into **Inbox / List**;
2. type one task name and, optionally, a `YYYY-MM-DD` due date;
3. add, complete, reopen, edit, and delete with the keyboard;
4. opt into scheduling detail only when it becomes useful; and
5. understand whether the work is safely stored on this device.

This audit records the behavior on `c36f3d7f17bd4e8f1ee284f85762abdb8ff5da13`
after the v0.1.0 packaging and recovery work. Findings are deliberately split
into child issues. This document does not turn the audit PR into a bundle of
unrelated product changes.

## Evidence collected

- The production web host passed `npm run build` and the real WASM presentation
  contract. The fixture starts in Inbox/List, creates a task with an optional due
  date, enables Full CPM explicitly, completes and reopens the task, restores a
  snapshot, and deletes the task.
- Both generated React themes contain the same List-first structure and controls.
  The light and dark Mosaic styles were inspected at desktop and compact-width
  constraints. The absence of a responsive rail/composer branch is itself the
  compact-window finding in #13692.
- A fresh `native-complete` WinUI project was emitted with the current
  `task-mosaic-app` Rust library. Its emitted-control contract passed, it
  published as a self-contained `Trestle.exe`, and UI Automation drove task
  creation, the optional due date, Rust scheduling, completion, reopening,
  deletion, and persisted restart restoration.
- The web host's startup and persistence paths were inspected directly. React is
  mounted only after WASM and storage initialization; storage fallback and
  corrupt-snapshot recovery currently report no user-visible state.

The in-app browser client failed before page JavaScript could execute, so no
browser-pane screenshots or visual claims are included. Web conclusions below
are limited to production compilation, executable WASM tests, generated output,
and authored layout/style inspection.

## Acceptance result

| Requirement | Result | Evidence and follow-up |
| --- | --- | --- |
| Clear List-first launch with one task-name field and optional due date | Partial | Inbox/List, `What needs doing?`, `Due (optional)`, and `Add task` are present. The task area is blank and the name field does not receive initial focus: #13687. |
| Create, complete, reopen, edit, and delete without project-management terminology | Partial | Create/complete/reopen/delete and direct name/due-date edits are engine-backed from List. The completion control still exposes only `○`/`✓` as its name (#13691). |
| Scheduling is progressively disclosed | Pass | New projects start in Board complexity; List details omit CPM fields until Full CPM is selected. A person can keep using name, due date, completion, and deletion without opening Timeline or Sheet. |
| Useful empty, loading, validation, overdue, and persistence-recovery states | Partial | Overdue is a plain-language row chip. Empty guidance/focus shipped in #13706. Composer validation is now visible, atomic, accessible, and focus-safe (#13689). Blank startup/failure remains #13695; persistence fallback and recovery remain console-only (#13690). |
| Local-only storage and backup location are discoverable | Fail | The release archives carry `LOCAL-DATA.txt` and the repository has an exact operations contract, but the running UI does not link or summarize it: #13690. |
| Both themes and representative desktop/compact web windows | Partial | Both authored themes build and mirror the same semantic structure. Desktop constraints are coherent; the always-visible 236 px rail and single-row fixed-width composer violate the documented compact behavior: #13692. No browser-pane screenshot evidence was available in this run. |
| At least one locally testable native backend | Pass | Fresh strict WinUI emit, control-contract validation, self-contained publish, and UI Automation lifecycle all passed against the current Rust adapter. |

## Prioritized refinement backlog

The next implementation work should preserve one focused issue per pull request:

1. [#13687](https://github.com/adhithyan15/coding-adventures/issues/13687) —
   empty-state guidance and deterministic first-task focus.
2. [#13689](https://github.com/adhithyan15/coding-adventures/issues/13689) —
   visible, focus-safe composer validation.
3. [#13691](https://github.com/adhithyan15/coding-adventures/issues/13691) —
   descriptive completion-control names.
4. [#13690](https://github.com/adhithyan15/coding-adventures/issues/13690) —
   local-storage, fallback, recovery, and backup discoverability.
5. [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695) —
   visible startup loading and failure states.
6. [#13692](https://github.com/adhithyan15/coding-adventures/issues/13692) —
   compact-window rail and composer behavior.

This order puts the first minute of use ahead of advanced editing and settings,
while keeping data-loss warnings above responsive polish.

## Re-run checklist

After the child issues land, repeat this audit from a fresh storage identity:

- light and dark at approximately 1440 x 900 and 800 x 600;
- keyboard-only first task, invalid task, invalid due date, edit, complete,
  reopen, expand scheduling, and delete;
- an overdue task and a restored task;
- durable-storage failure and corrupt-snapshot recovery; and
- one strict generated native host using its accessibility/automation tree.

Do not convert absence of console errors, successful compilation, or an engine
snapshot test into a visual or keyboard-accessibility claim.
