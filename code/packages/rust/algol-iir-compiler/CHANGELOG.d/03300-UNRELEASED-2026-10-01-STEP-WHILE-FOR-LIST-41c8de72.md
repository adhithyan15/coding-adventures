## Fixed

- Preserve a finite step element's exact control-variable exit snapshot when its body reads but does not write the controlled variable, allowing a following bounded `while` element in the same ALGOL `for` list to remain statically analyzable.
