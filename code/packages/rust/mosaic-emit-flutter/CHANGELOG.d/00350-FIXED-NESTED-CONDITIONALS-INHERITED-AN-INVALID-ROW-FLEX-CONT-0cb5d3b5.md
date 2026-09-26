### Fixed - nested conditionals inherited an invalid Row flex context (#13725)

Multi-widget `If`/`Else` branches introduce an intermediate `Column`. Their
nested widgets now use that Column as their parent context instead of
incorrectly inheriting the outer Row context, preventing `Expanded(TextField)`
from being emitted under unbounded vertical constraints. Direct-Row
conditionals also wrap the generated Column in `Flexible` when no explicit
branch flex is authored and the containing Row has bounded width, giving the
Column a finite horizontal constraint without introducing flex into a nested,
shrink-wrapped Row. A generated-project widget test pumps both nested branches
to keep the runtime layout valid.

