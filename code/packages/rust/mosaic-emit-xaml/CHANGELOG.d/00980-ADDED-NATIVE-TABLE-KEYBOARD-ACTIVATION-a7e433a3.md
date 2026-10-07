### Added — native table keyboard activation

Native table arrows invoke the target cell's unique authored container action
using the current row VM, then reacquire focus after adapter updates. Inputs
retain caret navigation; tables without an action keep focus-only behavior.
Ambiguous multiple actions fail explicitly instead of choosing a descendant.
