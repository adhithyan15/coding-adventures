# TaskApp repeated draft navigation acceptance

Extend the native XAML smoke with three Board/List round trips. Before each
round trip, focus the live composer and write a distinct unsaved draft. After
returning to List, resolve the live editor again and require its value to match
without rewriting it. Require the selected view to render after each action
and fail immediately on process exit. Then continue the existing task lifecycle
and persisted restart checks using the ordinary smoke task.

This validates draft retention through repeated conditional teardown and
recreation. It does not claim automatic focus restoration, keyboard caret,
IME composition, screen-reader acceptance or release packaging. Exercise the
same test against the full-package eager and repaired deferred Release hosts.
No generated application source changes and no weakening of existing checks.

## Validation

On 2026-11-03, the same script passed against the retained full-package eager
Release x64 host and repaired deferred Release x64 host at 144 DPI, each with
its own fresh MOSAIC_APP_STATE_PATH. Both passed three distinct draft round
trips, task creation/scheduling/complete/reopen/delete, and persisted restart.
The deferred host uses the #18886 emitter repair with the retained runtime DLL;
this is host interaction evidence, not a fresh packaged/downloaded release.
PowerShell parser validation and git diff --check passed. Security review passed.
