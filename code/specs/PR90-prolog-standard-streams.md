# PR90 - Prolog Standard Streams And Stream Options

## Overview

PR90 closes the practical standard-stream portion of the host runtime gap. The
VM already supported bounded file-backed streams; this batch adds default
standard stream aliases and a richer accepted `open/4` option subset that can
round-trip through `stream_property/2`.

This batch covers:

- `user_input`
- `user_output`
- `user_error`
- `reposition/1`
- `eof_action/1`
- `buffer/1`
- `close_on_abort/1`

The behavior is exposed through `logic-builtins`, adapted from source-level
Prolog calls by `prolog-loader`, and covered through both structured and
bytecode Prolog VM execution.

## Semantics

`user_input`, `user_output`, and `user_error` are always-open stream aliases.
The initial current input is `user_input`; the initial current output is
`user_output`. Current stream predicates can still select ordinary opened file
streams using `set_input/1` and `set_output/1`.

The standard output aliases are console-backed:

- writes to `user_output` write to `sys.stdout`
- writes to `user_error` write to `sys.stderr`

The default `user_input` stream is an empty bounded text stream. It is available
for metadata and EOF behavior without blocking on an interactive terminal.

`open/4` accepts these additional finite options:

- `reposition(true|false)`
- `eof_action(eof_code|error|reset)`
- `buffer(full|line|false)`
- `close_on_abort(true|false)`

Accepted options are exposed as `stream_property/2` metadata. They do not yet
implement every external Prolog runtime behavior behind those options.

## Boundaries

Standard stream aliases are process-global host resources, so they are not
closed by `close/1`. Richer binary stream services and the rest of the ISO/SWI
stream option surface remain future host-runtime work.
