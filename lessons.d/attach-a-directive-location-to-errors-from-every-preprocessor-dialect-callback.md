---
category: Compiler / VM / language pipeline
---

# Attach a directive location to errors from every preprocessor dialect callback

The PREP01 engine located `prepare_condition` errors but returned
`eval_condition` errors unchanged. A real C file with malformed `#if 1 &&`
therefore reached the frontend as line 0, column 0 even though the engine knew
the directive's position. At every dialect callback, preserve a position the
dialect already supplied and otherwise attach the current directive position
before returning the error. Verify this through a file-input frontend, not
only through a direct dialect unit test.
