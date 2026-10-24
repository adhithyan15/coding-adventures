---
category: Repo policy / workflow reminders
---

# Inspect downloaded artifacts before format-specific pipelines

A certificate download was piped directly into `openssl x509 -inform DER`
after assuming the remote endpoint returned DER. The endpoint returned a
different representation, so the downstream decoder failed without revealing
the actual response format. When consuming a remote artifact, first inspect
its status, content type, and leading bytes (or save it in a temporary
directory and run `file`) before choosing a format-specific parser.
