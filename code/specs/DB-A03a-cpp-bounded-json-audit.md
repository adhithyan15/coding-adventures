# DB-A03a — C++ bounded-json Admission Audit for the DB01 Test Adapter

## Disposition

The header-only C++17 parser is suitable for a **test-only** DB01 fixture
reader, provided the reader performs its own exact-schema validation. The
parser gives the reader the required syntax, token-kind, duplicate-member,
UTF-8, trailing-input, and resource-limit observations. It does not validate
DB01 keys, error names, operation shapes, or state semantics, and it does not
reject duplicate object names by itself. No C NAND binding or runtime JSON
dependency is needed.

## Pinned source and environment

Audit test checkout: `ddc2d8032cdcdcd56b64b961b9314d91aa1951af`. While this
audit was in progress, HEAD advanced to `5380d8dc03f24889aecbf2da5f842de2d4561ec7`.
The bounded-json package, selected NAND fixture, and C harness had no changes
between those revisions and remain clean in the current checkout. C++ package
files were hashed from the working tree after confirming clean status.

| Input | SHA-256 |
|---|---|
| Tracked C++ bounded-json tree (15 files, hashed in repository path order) | `b588c0e5adf4de37fe0b283f12e2f8e66844984fe5f2b8ddecab574ef839c767` |
| `include/bounded_json.hpp` | `599a4095584e73733a23741b834bef5a03cc28f9f74b0b1c69648dba1f6f0846` |
| `tests/json_test.cpp` | `9532a4a62ea22f0da951a1589f238f2af1e4893c7f00eb5eae74cebda6618ed1` |
| `tests/consumer.cpp` | `c861503131980dd201392788c9e0a903bb691b920faae8319d4504ace444026f` |
| `tools/run.sh` | `ca9f8138e58eb4e42e833572c930e191c89fc50f82f25a9ecc88202b64c01653` |
| `BUILD` | `a5a4fc606d04a90b9ec704f5697c31b0f8ba5eaad95539d44c56a41501b8f0c9` |
| DB01 NAND fixture | `6bf9f32a47af48da802a56d3d5104b48a8601286a31c62cc43c78c3e0baaf05e` |
| C `iso_test.h` | `614116c46ba4e08b7cbfe288edaad492487d2f324b1a6075b9b431cf5c8b7ac8` |
| C `iso-lib.sh` | `68349cd2b210e0440a853f89dec473b4db6d768a85f24f6270daa21a7ee85b2b` |

The initial audit used `g++ (Debian 14.2.0-19) 14.2.0`. A later strict
Clang++ 19.1.7 run also passed all 474 checks against the pinned package. The
fixture is 6171 bytes and has six top-level object members.

The tree digest is SHA-256 over each full repository-relative path encoded as
UTF-8, one NUL byte, and the file's raw 32-byte SHA-256. Concatenate those
records in ascending path order.

## Commands and observations

The package's strict native test entry point passed with 474 checks and zero
failures. Its output directory was outside the checkout:

```sh
cd code/packages/cpp/bounded-json
probe_dir=$(mktemp -d /tmp/db-a03a-tests.XXXXXX)
trap 'rm -rf "$probe_dir"' EXIT
ISO_BUILD_DIR="$probe_dir/build" ISO_REQUIRE=g++ env -u CI sh tools/run.sh
```

The two native translation units also passed under GCC AddressSanitizer and
UndefinedBehaviorSanitizer, again with 474 checks and zero failures:

```sh
cd code/packages/cpp/bounded-json
san_dir=$(mktemp -d /tmp/db-a03a-sanitize.XXXXXX)
trap 'rm -rf "$san_dir"' EXIT
g++ -std=c++17 -pedantic-errors -Wall -Wextra -Werror \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -I include -I ../../c/iso-harness/include \
  tests/json_test.cpp tests/consumer.cpp -o "$san_dir/bounded-json-sanitized"
ASAN_OPTIONS=detect_leaks=1 "$san_dir/bounded-json-sanitized"
```

A separate strict C++17 probe was compiled in `/tmp` and run against the
canonical NAND fixture. It verified all of the narrow reader dependencies:

| Reader requirement | Observation |
|---|---|
| Tight-bounded canonical parse | `Options` set to 64 KiB bytes, depth 64, 16384 nodes, and 8 MiB allocation parsed the 6171-byte fixture as an object with six members. |
| Exact numeric token kinds | Integer `7`, boolean `true`, floating `7.0`, and exponent `7e0` had distinct `integer`, `boolean`, and `floating` kinds. |
| Duplicate handling | Parsing `{"x":1,"x":2}` succeeded and retained two same-named members in order. The fixture adapter must reject duplicates itself. |
| UTF-8 and trailing bytes | Invalid raw UTF-8 returned `Status::utf8`; `null null` returned `Status::syntax`. |
| Resource failures | Input cap 3 on `null` returned `byte_limit`; depth 0 on `[]` returned `depth_limit`; node cap 0 on `null` returned `node_limit`; allocation cap 0 on `null` returned `memory_limit`. |

Reproducible probe source and command follow. Run from the repository root:

```sh
probe_dir=$(mktemp -d /tmp/db-a03a-probe.XXXXXX)
trap 'rm -rf "$probe_dir"' EXIT
g++ -std=c++17 -pedantic-errors -Wall -Wextra -Werror \
  -I code/packages/cpp/bounded-json/include -x c++ \
  -o "$probe_dir/probe" - <<'CPP'
#include "bounded_json.hpp"
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
namespace bj = ca::bounded_json;
static const bj::Value* find(const bj::Value& object, std::string_view name) {
    for (std::size_t i = 0; i < object.size(); ++i) {
        auto entry = object.member(i);
        if (entry.first == name) return entry.second;
    }
    return nullptr;
}
int main(int argc, char** argv) {
    if (argc != 2) return 2;
    std::ifstream fixture(argv[1], std::ios::binary);
    std::string input((std::istreambuf_iterator<char>(fixture)), std::istreambuf_iterator<char>());
    bj::Options options; options.bytes = 64 * 1024; options.depth = 64;
    options.nodes = 16384; options.memory = 8 * 1024 * 1024;
    auto canonical = bj::parse(input, options);
    if (!canonical || canonical.document->root().kind() != bj::Kind::object ||
        canonical.document->root().size() != 6) return 10;
    auto scalar = bj::parse("{\"i\":7,\"b\":true,\"f\":7.0,\"e\":7e0}", options);
    if (!scalar || find(scalar.document->root(), "i")->kind() != bj::Kind::integer ||
        find(scalar.document->root(), "b")->kind() != bj::Kind::boolean ||
        find(scalar.document->root(), "f")->kind() != bj::Kind::floating ||
        find(scalar.document->root(), "e")->kind() != bj::Kind::floating) return 11;
    auto duplicate = bj::parse("{\"x\":1,\"x\":2}", options);
    if (!duplicate || duplicate.document->root().size() != 2 ||
        duplicate.document->root().member(0).first != duplicate.document->root().member(1).first) return 12;
    std::string invalid_utf8; invalid_utf8.push_back('"');
    invalid_utf8.push_back(static_cast<char>(0xff)); invalid_utf8.push_back('"');
    if (bj::parse(invalid_utf8, options).error.kind != bj::Status::utf8 ||
        bj::parse("null null", options).error.kind != bj::Status::syntax) return 13;
    bj::Options cap; cap.bytes = 3;
    if (bj::parse("null", cap).error.kind != bj::Status::byte_limit) return 14;
    cap = {}; cap.depth = 0;
    if (bj::parse("[]", cap).error.kind != bj::Status::depth_limit) return 15;
    cap = {}; cap.nodes = 0;
    if (bj::parse("null", cap).error.kind != bj::Status::node_limit) return 16;
    cap = {}; cap.memory = 0;
    if (bj::parse("null", cap).error.kind != bj::Status::memory_limit) return 17;
    std::cout << "canonical_bytes=" << input.size() << " root_members=6 "
                 "int/bool/float=distinct duplicate_members=preserved "
                 "UTF8=reject trailing=reject caps=byte/depth/node/allocation reject\n";
}
CPP
"$probe_dir/probe" code/specs/fixtures/database-media-v1/nand-flash.json
```

Observed probe output: `canonical_bytes=6171 root_members=6 int/bool/float=distinct duplicate_members=preserved UTF8=reject trailing=reject caps=byte/depth/node/allocation reject`.

## Admission conditions and uncovered claims

For a DB01 test reader, use the C++ header directly as a test-only include
(`../bounded-json/include`) and keep package production headers/runtime
independent. Read the fixture with a 64 KiB file-size check before allocation,
then parse with the tightened options above. Walk objects by indexed members;
reject repeated keys and require exact key sets at each schema level. Check
`Kind` before every accessor, require integer tokens where the schema says
integer, and validate all operation/error names and post-operation state
explicitly. Keep `Value` and string views inside the lifetime of the owning
`Document`.

This audit did **not** implement or test a DB01 schema adapter, duplicate-key
rejection policy, fixture replay, native NAND behavior, malformed DB01 schema
cases, or parser fuzz/sanitizer behavior beyond the stated probe and package
suite. It does not review the C parser, claim general C++ JSON correctness,
or make `bounded-json` a runtime dependency. The package suite and sanitizer
run cover the parser's own claims; the DB01 adapter must add its own tests for
schema shape, duplicate rejection, exact error names/messages, and full state
comparison after each operation.
