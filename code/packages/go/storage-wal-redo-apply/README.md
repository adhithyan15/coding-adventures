# storage-wal-redo-apply

Replays an admitted committed redo plan into data pages. It completes one full page write and its durability barrier before moving to the next image. A refusal while applying reports the current page, source record LSN, successful block prefix, and prior acknowledged image count; a lost barrier response may leave bytes durable while returning an interruption.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
