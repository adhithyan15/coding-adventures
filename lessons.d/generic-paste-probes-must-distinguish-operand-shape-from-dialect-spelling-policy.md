---
category: Compiler / VM / language pipeline
---

# Generic paste probes must distinguish operand shape from dialect spelling policy

A generic PREP01 suffix-paste probe expected `x ## ##` to fail before a test
hook was called. The engine resolves one parameter and one literal token;
the C dialect separately rejects invalid identifier operands and operator
combinations. A permissive test hook accepted the concatenated spelling, so
the probe had accidentally tested a C policy through a non-C dialect.

Use two parameters, two literals, or an operator at the replacement-list edge
to exercise the engine's supported operand shape. Exercise C spelling and
multiple-operator rejection through the real C dialect. Keep counted hooks
for the generic budget ordering and returned-token validation contracts.
