"""Parrot — a demonstration REPL that echoes back whatever you type.

What Is Parrot?
---------------
Parrot is a minimal demonstration program built on top of the
``coding-adventures-repl`` framework. Its sole purpose is to show how
the three pluggable components — Language, Prompt, and Waiting — are
wired together into a working interactive program.

A parrot repeats what it hears. This REPL repeats what you type.

Usage
-----
Run interactively::

    parrot          # if installed via pip/uv
    python -m parrot.main

Programmatic use (testing, embedding)::

    from coding_adventures_repl import EchoLanguage, SilentWaiting, run_with_io
    from parrot.prompt import ParrotPrompt

    outputs: list[str] = []
    inputs = iter(["hello", ":quit"])

    run_with_io(
        language=EchoLanguage(),
        prompt=ParrotPrompt(),
        waiting=SilentWaiting(),
        input_fn=lambda: next(inputs, None),
        output_fn=outputs.append,
    )
    # outputs now contains the banner, "hello", and the final banner.
"""
