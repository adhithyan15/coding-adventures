### Changed — lesson duration estimates are computed once per lesson

`estimateLessonDuration` now caches its result per parsed lesson in a `WeakMap`. It is a pure function of one lesson, and the same lesson objects were estimated again by curriculum validation and by every gap-report build. The markdown link stripper now copies plain text a run at a time instead of one character at a time.

The full-corpus gap report is byte-identical before and after. On 15,723 lessons, a first build goes from 8.6s to 8.2s and a repeat build over the same lessons from 8.4s to 6.6s. Tests build that report several times per file.

The cached estimate and its `reasons` array are frozen, because every caller now shares them. A review of this change also found a pre-existing crash: an unclosed "[" followed by a few hundred thousand characters overflowed the call stack, because the buffered label was spread into `push()` one argument per character. The label is now joined instead, and a 500,000-character test pins the fix.

These are the two cheapest wins from a CPU profile of the report. The larger remaining costs are the continuity walk (~1.7s) and the script-closure measure (~0.8s), and they are recorded for follow-up.
