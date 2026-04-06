"""SilentWaiting — the no-op waiting plugin.

Philosophy: Do Nothing, Do It Well
------------------------------------
SilentWaiting implements the :class:`~coding_adventures_repl.Waiting` ABC
with every callback as a true no-op.  It is the right choice when:

1. **Evaluation is fast** — If the language evaluator responds in
   microseconds (as :class:`~coding_adventures_repl.EchoLanguage` does),
   any visual waiting indicator would appear and disappear too quickly to
   be useful and might even cause visible flickering.

2. **Testing** — Spinner animations and timed displays complicate test
   output.  SilentWaiting keeps test runs clean and deterministic.

3. **Non-interactive use** — When the REPL is driven by piped input or used
   programmatically, visual indicators are pure noise.

4. **Baseline** — It is the default waiting plugin.  Any new REPL session
   starts silent and you opt in to visual feedback by swapping this out.

The Tick Rate
-------------
Even though :meth:`tick` is a no-op, :meth:`tick_ms` returns ``100``.
This controls how often the loop polls for thread completion via
``thread.join(timeout=0.1)``.  100 ms means the loop checks 10 times per
second — responsive enough that a fast evaluator doesn't wait needlessly,
slow enough to avoid busy-looping.

State
-----
:meth:`start` returns ``None``.  :meth:`tick` accepts and ignores it,
returning ``None``.  :meth:`stop` accepts and ignores it.  There is nothing
to track.
"""

from __future__ import annotations

from typing import Any

from coding_adventures_repl.waiting import Waiting


class SilentWaiting(Waiting):
    """No-op waiting plugin — does nothing while eval runs.

    All lifecycle methods are no-ops.  The only meaningful value is the
    100 ms tick interval which controls the eval-completion poll rate.

    Examples
    --------
    >>> w = SilentWaiting()
    >>> state = w.start()
    >>> state is None
    True
    >>> state = w.tick(state)
    >>> state is None
    True
    >>> w.tick_ms()
    100
    >>> w.stop(state)   # returns None, no side effects
    """

    def start(self) -> None:
        """Return ``None`` — there is no state to initialise.

        The loop stores whatever is returned here and passes it to every
        subsequent :meth:`tick` and to :meth:`stop`.  For SilentWaiting that
        value is always ``None``.

        Returns
        -------
        None
        """
        # No state to initialise — return None as a placeholder that
        # documents "this waiting plugin is stateless."
        return None

    def tick(self, state: Any) -> None:
        """Accept the state, do nothing, return ``None``.

        The loop calls this every :meth:`tick_ms` milliseconds while the
        evaluator thread is running.  We simply ignore both the state and the
        tick itself.

        Parameters
        ----------
        state:
            Ignored.

        Returns
        -------
        None
        """
        # Nothing to animate. Accept the (None) state and hand it back
        # unchanged. The loop will call this every tick_ms() milliseconds;
        # we simply ignore it.
        return None

    def tick_ms(self) -> int:
        """Return ``100`` — poll for eval completion ten times per second.

        This is the only knob that matters for SilentWaiting: it controls
        how quickly the loop responds when eval finishes between ticks.

        100 ms (10 polls/second) balances responsiveness against CPU waste.

        Returns
        -------
        int
            ``100``
        """
        return 100

    def stop(self, state: Any) -> None:
        """Do nothing — there is nothing to clean up.

        Parameters
        ----------
        state:
            Ignored.
        """
        # Nothing to clean up.  The silent waiting plugin leaves no visual
        # artifacts on screen and holds no resources to release.
        return None
