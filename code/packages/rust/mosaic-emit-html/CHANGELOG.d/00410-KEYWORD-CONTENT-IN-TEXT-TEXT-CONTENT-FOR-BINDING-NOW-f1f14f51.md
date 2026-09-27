- **Keyword content in Text** — `Text (content: <For-binding>)` now
  lowers to the same `{{binding}}` Mustache form that SlotRef
  content uses, so cells iterated by a For actually render the
  bound value rather than blank.

Together these unblock the L10 VisiCalc Grid migration on the HTML
backend.

3 new tests, total 82 (was 79):
