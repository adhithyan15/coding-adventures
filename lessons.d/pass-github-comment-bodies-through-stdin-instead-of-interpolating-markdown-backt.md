---
category: Repo policy / workflow reminders
---

# Pass GitHub comment bodies through stdin instead of interpolating Markdown backticks into a shell command

Four durable GitHub checkpoint comments were assembled as shell command
strings. `JSON.stringify` added double quotes around each body, but the bodies
contained Markdown backticks. Zsh still performed command substitution inside
those double quotes, tried to execute a commit hash and field names, and posted
comments with the code-formatted text missing.

JSON string escaping is not shell escaping. For externally visible prose, pass
the body on standard input with `gh issue comment --body-file -`, or write a
temporary file through a safe tool and pass its explicit path. If a short body
must be a shell literal, single-quote it and first prove it contains no single
quote. Never interpolate user-visible Markdown into a double-quoted command.

When this happens, inspect the external result and post a clearly labelled
correction rather than pretending the malformed comment is complete.
