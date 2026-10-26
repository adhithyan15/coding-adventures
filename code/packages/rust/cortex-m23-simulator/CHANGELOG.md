# Changelog

## 0.1.0

- Add the bounded Cortex-M23 functional core with checked Armv6-M-compatible
  instruction execution.
- Add explicit Security Extension configuration, banked Secure/Non-secure
  state, eight-region SAU attribution, and checked transition hooks.
- Add typed atomic lifecycle failures and bounded complete-state traces.
- Keep inherited validation scratch non-observable and restore it whenever a
  candidate or bounded lifecycle operation is rejected.
