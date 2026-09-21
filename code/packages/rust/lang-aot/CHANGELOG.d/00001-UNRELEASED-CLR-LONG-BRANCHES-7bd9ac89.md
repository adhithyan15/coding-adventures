## Unreleased — promoted CLR long branch execution (CLR10)

Add a direct builder dev-dependency and an integration test that independently
checks promoted br/brfalse/brtrue opcode and displacement bytes, then executes
both condition outcomes in the simulator. Default source and strict scalar
control-flow routing remain unchanged.
