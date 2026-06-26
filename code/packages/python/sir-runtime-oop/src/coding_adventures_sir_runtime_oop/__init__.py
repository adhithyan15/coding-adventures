"""coding-adventures-sir-runtime-oop — OOP runtime for SIR-emitted Python.

SIR backends translate most constructs to **native** Python.  Ruby-style object
orientation is the exception: the Ruby->SIR frontend **hoists every method to a
detached top-level function with no receiver (``self``)**, so an emitted method
has no ``self`` for instance variables and a class-variable write carries no
enclosing-class context — native attribute access (``self.x``) is impossible.
This package supplies the missing object model, imported by the emitted module::

    from coding_adventures_sir_runtime_oop import define_class as _sir_oop_define_class
    _sir_oop_define_class("Dog", "Animal")     # class Dog < Animal

It implements **SIR** semantics (not any one source language's), so a future
JavaScript -> SIR -> Python path reuses it.  See ``code/specs/sir-runtime.md``.

**v0 limitation:** the frontend does not thread receivers, so the current-self is
a process-global stack and class variables share one namespace keyed by bare
name — single-instance / single-class faithful, full multi-object semantics
pending frontend receiver threading.
"""

from __future__ import annotations

from .oop import (
    SirInstance,
    Val,
    call_method,
    case_eq,
    class_of,
    cvar_get,
    cvar_set,
    define_class,
    define_method,
    is_a,
    ivar_get,
    ivar_set,
    new_instance,
    pop_self,
    push_self,
    reset_oop,
    superclass_of,
    sym_to_proc,
)

__all__ = [
    "SirInstance",
    "Val",
    "call_method",
    "case_eq",
    "class_of",
    "cvar_get",
    "cvar_set",
    "define_class",
    "define_method",
    "is_a",
    "ivar_get",
    "ivar_set",
    "new_instance",
    "pop_self",
    "push_self",
    "reset_oop",
    "superclass_of",
    "sym_to_proc",
]
