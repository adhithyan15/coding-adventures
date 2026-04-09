# wasm-runtime (Perl)

Complete WebAssembly 1.0 runtime for Perl. Composes the WASM module parser,
validator, and execution engine into a single user-facing API. Handles parsing,
validation, instantiation (memory allocation, global initialization,
data/element segments), function calling, and WASI host functions.

## Features

- Full parse → validate → instantiate → call pipeline
- WASI Tier 1: `proc_exit`, `fd_write`
- WASI Tier 3: `args_sizes_get`, `args_get`, `environ_sizes_get`, `environ_get`,
  `clock_res_get`, `clock_time_get`, `random_get`, `sched_yield`
- Pluggable clock and random sources (inject fakes for testing)

## Quick Start

```perl
use CodingAdventures::WasmRuntime;

my $runtime = CodingAdventures::WasmRuntime->new();
my $results = $runtime->load_and_run($wasm_bytes, 'square', [5]);
# $results = [25]
```

## WASI Support

Use `WasiStub` as the host interface for programs that import WASI functions:

```perl
use CodingAdventures::WasmRuntime;

my $wasi = CodingAdventures::WasmRuntime::WasiStub->new(
    args   => ['myapp', '--verbose'],
    env    => { HOME => '/home/user', PATH => '/usr/bin' },
);

my $rt       = CodingAdventures::WasmRuntime->new(host => $wasi);
my $module   = $rt->load($wasm_bytes);
my $instance = $rt->instantiate($module);

# Wire memory so WASI memory-accessing functions can operate:
$wasi->set_memory($instance->{memory});

my $result = $rt->call($instance, '_start', []);
```

### Pluggable clock and random

```perl
# Use a fake clock for deterministic tests:
package FakeClock;
sub new { bless {}, shift }
sub realtime_ns  { 1_000_000_000 }
sub monotonic_ns { 500_000_000 }
sub resolution_ns { 1_000_000 }

# Use a fake random source:
package FakeRandom;
sub new { bless {}, shift }
sub fill_bytes { my ($self, $n) = @_; return [(0xDE) x $n] }

package main;
my $wasi = CodingAdventures::WasmRuntime::WasiStub->new(
    clock  => FakeClock->new(),
    random => FakeRandom->new(),
);
```

## Dependencies

- wasm-leb128
- wasm-types
- wasm-opcodes
- wasm-module-parser
- virtual-machine
- wasm-validator
- wasm-execution
- Encode (core Perl module)
- Time::HiRes (core Perl module)

## Development

```bash
# Run tests
bash BUILD
```
