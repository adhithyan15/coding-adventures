- **A press is not a drag.** A 5px movement threshold and a primary-button check
  gate the grab. Without them, clicking a card grabbed it and immediately dropped
  it on its own enclosing container — a spurious reorder whose position depended
  on where in the card you clicked, and any button nested in a card was unusable.
