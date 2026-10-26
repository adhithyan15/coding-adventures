# Native C++ JSON before native C++ training

A checkpoint loader must distinguish JSON syntax from the training schema.
NN50 provides only the first layer: owned text values with bounded allocation,
strict UTF-8, duplicate-preserving object members and lexical integer metadata.
It does not train a model or decide whether an object is a valid checkpoint.

For example, `{"step":1,"step":2}` is valid JSON containing two ordered
members. Preserve both so a later closed schema can reject the duplicate.
Likewise `1`, `1.0`, true and null remain four different kinds. An embedded NUL
in a key is data with a length, not a C string terminator.

The document owns a list of every allocated node, independently of the syntax
tree's parent/child edges. Destroying that list releases even partially attached
nodes without recursive destruction. Every node and dynamic name/string buffer
uses one capped memory resource. Temporary growth counts old and replacement
buffers simultaneously. This measures allocation requests, not merely payload
length or process RSS; fixed owner/runtime overhead is documented separately.

Build from `code/packages/cpp/bounded-json` with its shared ISO BUILD front.
Two translation units check header self-containment and linkage. The injected
upstream allocator fails every observed allocation position in both valid and
malformed trees. GCC/Clang tests, GCC coverage, Clang ASan and an installed CMake
consumer pass locally. Native Windows compilation and final CI remain pending;
a local SDK missing `crtdbg.h` is not evidence of a parser compile failure.

The independent [C++ package](../../packages/cpp/bounded-json/README.md) does not
call the C parser. See [NN50](../../specs/NN50-cpp-bounded-json-foundation.md).
Native C++ dense training and actual seven-native exchange are separate work.
