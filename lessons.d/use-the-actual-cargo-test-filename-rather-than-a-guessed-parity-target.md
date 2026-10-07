---
category: Testing & coverage
---

# Use the actual Cargo test filename rather than a guessed parity target name

The validation command guessed cli_surface_parity, but Cargo's integration target is the actual filename cli_surface.rs, exposed as cli_surface. Cargo rejected the whole command before running either selected suite. Read the existing filename or Cargo's available-target error, then rerun the corrected command; the rejected invocation provides no test evidence.

The first lesson-fill step also guessed a full title-derived filename instead of waiting for lessons.py new's returned, truncated filename. Use that exact returned path for the dependent edit and run lesson validation after filling it.
