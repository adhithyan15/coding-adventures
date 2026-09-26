# Use defineProperties to shadow getter-only prototype accessors in forgery tests

`Object.assign` uses ordinary assignment semantics, so it cannot create an own
property when the prototype exposes a getter-only accessor with the same name.
That made a prototype-forgery regression test fail before reaching the code it
was meant to exercise. Use `Object.defineProperties` with explicit data-property
descriptors when a test deliberately shadows getter-only prototype accessors;
this constructs the hostile object without invoking the inherited setters.
