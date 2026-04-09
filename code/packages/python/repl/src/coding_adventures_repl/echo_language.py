"""EchoLanguage — the simplest possible Language implementation.

What It Does
------------
EchoLanguage mirrors the user's input back as the result, unchanged.
It is the "hello world" of language plugins:

    > hello
    hello
    > 42
    42
    > :quit
    (session ends)

Why Does This Exist?
--------------------
1. **Testing** — The REPL loop is independent of any real language.  To test
   the loop's plumbing (I/O injection, waiting integration, error handling)
   without a real evaluator, we need a trivial stand-in.  EchoLanguage fills
   that role perfectly.

2. **Documentation** — It shows exactly what the Language ABC contract looks
   like when implemented.  A reader new to the codebase can understand the
   interface by reading a dozen lines here before moving on to a more complex
   implementation.

3. **Demos** — A standalone REPL demo that "works" without any language
   installed is useful for presentations and early integration testing.

The Quit Convention
-------------------
The string ``":quit"`` is treated as a special sentinel that signals the user
wants to end the session.  This mirrors the convention used in many terminal-
based REPLs (Python uses ``quit()`` or ``exit()``; Ruby's irb uses ``quit``).

The exact sentinel is intentionally a plain string — there is no magic keyword
parsing in the loop itself.  If you implement your own language and want a
different quit sequence (``:exit``, ``quit()``, ``\\q``, etc.), simply return
``"quit"`` for that input.
"""

from __future__ import annotations

from coding_adventures_repl.language import Language


class EchoLanguage(Language):
    """Echo the input back as the result.

    This is a concrete, minimal implementation of :class:`Language` that
    exists primarily for testing and demonstration purposes.

    Examples
    --------
    >>> lang = EchoLanguage()
    >>> lang.eval("hello world")
    ('ok', 'hello world')
    >>> lang.eval(":quit")
    'quit'
    """

    def eval(self, input: str) -> tuple[str, str | None] | str:
        """Echo the input back, or quit if the input is ``:quit``.

        Parameters
        ----------
        input:
            The raw text the user typed (newline already stripped).

        Returns
        -------
        tuple[str, str | None] | str
            - ``"quit"``          if *input* == ``":quit"``
            - ``("ok", input)``   for any other input — the text is mirrored
              back unchanged as the result value.

        Notes
        -----
        This implementation is intentionally stateless and thread-safe:
        it has no mutable instance variables, so concurrent calls are safe.
        """
        # The special sentinel ":quit" signals the loop to end the session.
        # Any other string is echoed back unchanged wrapped in an ok tuple.
        if input == ":quit":
            return "quit"

        # Wrap the echoed string in the success tuple so the loop can
        # distinguish "evaluation succeeded with a value" from "evaluation
        # succeeded with no output" (which would be ("ok", None)).
        return ("ok", input)
