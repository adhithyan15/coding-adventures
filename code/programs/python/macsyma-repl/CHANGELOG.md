# Changelog

## 0.1.0 — 2026-04-25

Initial release — Phase A.

- Interactive REPL invocable via ``python main.py`` or ``python -m main``.
- ``MacsymaLanguage`` plugin that ties together
  ``macsyma-{lexer,parser,compiler}``, the ``symbolic-vm`` evaluator,
  the ``macsyma-runtime`` backend, and the ``cas-pretty-printer``
  output formatter.
- ``MacsymaPrompt`` plugin showing ``(%iN) `` for the global prompt.
- ``Display(expr)`` / ``Suppress(expr)`` wrappers from the compiler are
  inspected before evaluation to decide whether to print results;
  identity-on-inner under the runtime backend.
- ``%``, ``%iN``, ``%oN`` resolve via the ``History`` table installed
  on the backend.
- ``quit;`` and ``:quit`` exit the session; parse / compile / runtime
  errors are caught and the loop continues.
