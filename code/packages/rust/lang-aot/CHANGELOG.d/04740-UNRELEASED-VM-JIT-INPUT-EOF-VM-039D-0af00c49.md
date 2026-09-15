## Unreleased — VM/JIT input/EOF (VM-039d)

Register input_more on the plain VM, JIT interpreter fallback and compiled
GenericCirJit backend, sharing the numeric/string input queue without consuming
bytes. The four FLOW-MATIC input/EOF rows now declare all seven standard
backends. A direct compiled-callback proof asserts compilation, invocation
count and no interpreter fallback; shared-queue tests prove stable peeks.

