### Fixed — Ruby Canonical Starlark BUILD Compatibility
- Ruby's Starlark stack now closes indented files in the specified token order,
  preserves `r`/`b`-leading identifiers, binds mixed keyword calls, and keeps
  defining-module globals across nested loads.
- The Ruby build tool injects the normalized v1 evaluation context, validates
  structured commands, and fails closed after Starlark classification instead
  of silently falling back to raw shell lines; evaluation errors redact the
  checkout root.

