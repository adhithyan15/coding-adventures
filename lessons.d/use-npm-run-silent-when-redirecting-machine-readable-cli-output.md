---
category: Repo policy / workflow reminders
---

# Use npm run --silent when redirecting machine-readable CLI output

`npm run` writes its package-and-script banner to standard output. Redirecting a
JSON-producing script through ordinary `npm run` therefore creates a file whose
first bytes are the banner rather than `{`, and the next JSON consumer fails.

Use `npm run --silent <script> -- <args> > output.json`, or invoke the built
entrypoint directly, whenever standard output is the machine-readable artifact.
Validate the result with a JSON parser before treating the command as a
successful measurement.
