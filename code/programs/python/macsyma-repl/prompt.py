"""``MacsymaPrompt`` — provides Maxima-style ``(%iN) `` prompts.

The prompt has a reference to the language plugin's :class:`History`
so it can use the next input index without duplicating state.
"""

from __future__ import annotations

from coding_adventures_repl import Prompt
from macsyma_runtime import History


class MacsymaPrompt(Prompt):
    """Renders ``(%iN) `` as the global prompt and ``        `` for continuation.

    The continuation prompt is unused by the current REPL framework
    (which doesn't have a ``needs_more?`` hook), but we ship one for
    forward-compatibility.
    """

    history: History

    def __init__(self, history: History) -> None:
        self.history = history

    def global_prompt(self) -> str:
        return f"(%i{self.history.next_input_index()}) "

    def line_prompt(self) -> str:
        # Maxima's continuation indent — eight spaces.
        return "        "
