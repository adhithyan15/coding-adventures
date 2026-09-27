---
category: Repo policy / workflow reminders
---

# Do not invoke repository tests through the code package name

On Windows I tried to run the lesson contract tests as
`python -m unittest code.scripts.tests.test_lessons`. Python resolved the
standard-library `code` module before the repository directory, so discovery
failed immediately with `AttributeError: module 'code' has no attribute
'scripts'`. That was a command-selection failure, not a product or test
failure.

Run repository script tests by file or by the documented discovery root:

```text
python code/scripts/tests/test_lessons.py
python -m unittest discover -s code/scripts/tests -p test_lessons.py
```

Do not infer a dotted module path merely from the filesystem path when a top-
level directory shares a name with a standard-library module. Read the
relevant lesson's exact command before improvising a platform-specific form.
