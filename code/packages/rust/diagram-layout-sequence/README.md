# diagram-layout-sequence

Backend-neutral layout for sequence diagrams. It converts participants and
ordered events into participant boxes, lifelines, message routes, notes, and
activation bars for `diagram-to-paint`. Nested control-block events become
depth-aware frames and labeled branch dividers without introducing paint or
backend concepts into semantic IR. Participant lifecycle events place dynamic
headers and bound their lifelines to explicit destruction markers.
Participant-group geometry encloses the lanes declared in Mermaid `box`
blocks and carries backend-neutral labels and optional fills into PaintScene.
