# paint-vm-ascii

Go terminal backend for `paint-instructions`.

This package renders the current Go `PaintScene` model to a plain string. The
Go paint IR is still rect-focused today, so the initial backend renders filled
rectangles as block characters and rejects unknown instruction kinds.
