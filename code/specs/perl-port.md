# Perl Port Specification

## Overview

Port the entire coding-adventures monorepo to Perl, making it the 7th supported
language alongside Python, Ruby, Go, Rust, TypeScript, and Elixir.

**Target:** Perl 5.40+ (native `class` syntax, subroutine signatures, `try/catch`)

---

## Motivation

Perl is one of the foundational scripting languages — it pioneered many of the
ideas that Python, Ruby, and JavaScript later adopted (regular expressions as
first-class citizens, CPAN as the model for package repositories, "There's More
Than One Way To Do It" philosophy). Adding Perl to the monorepo:

1. Exercises Perl's modern features (native `class` from 5.38+, signatures from
   5.36+, `try/catch` from 5.40+) which are largely unknown to developers who
   only know "legacy" Perl.
2. Tests portability of algorithms across a 7th paradigm (Perl's unique blend of
   procedural, OO, and functional styles).
3. Provides a practical exploration of Perl's package management ecosystem
   (CPAN, cpanm, cpanfile, Carton, Dist::Zilla).

---

## Perl Tooling Stack

| Concern            | Tool              | Equivalent in Python    |
|--------------------|-------------------|------------------------|
| Runtime            | Perl 5.40+ (mise) | Python 3.12+ (mise)    |
| Package repository | CPAN (MetaCPAN)   | PyPI                   |
| Dep declaration    | `cpanfile`        | `pyproject.toml`       |
| Lock file          | `cpanfile.snapshot`| `uv.lock`              |
| Installer          | `cpanm` / `cpm`   | `uv` / `pip`           |
| Build/release      | Dist::Zilla       | hatchling              |
| Testing            | Test2::V0 + prove | pytest                 |
| Linting            | Perl::Critic      | ruff                   |
| Formatting         | Perl::Tidy        | ruff format / black     |
| OOP                | native `class`    | built-in classes       |
| Type constraints   | Type::Tiny        | type hints + mypy      |
| Coverage           | Devel::Cover      | pytest-cov             |

---

## Package Structure

Every Perl package follows this layout:

```
code/packages/perl/package-name/
  BUILD                           # Test command (prove -l t/)
  README.md                       # Package documentation
  CHANGELOG.md                    # Keep a Changelog format
  cpanfile                        # Dependencies
  lib/
    CodingAdventures/
      PackageName.pm              # Main module
      PackageName/
        SubModule.pm              # Sub-modules (if needed)
  t/
    01-basic.t                    # Tests (Test2::V0)
    02-advanced.t
```

### Naming Convention

- **Directory:** `code/packages/perl/package-name/` (kebab-case)
- **Module namespace:** `CodingAdventures::PackageName` (CamelCase)
- **CPAN distribution name:** `CodingAdventures-PackageName`

### BUILD File Patterns

Standalone package (no internal deps):
```
prove -l t/
```

Package with internal dependencies:
```
prove -l -I ../dep-a/lib -I ../dep-b/lib t/
```

### cpanfile Pattern

```perl
requires 'perl', '5.040';

on 'test' => sub {
    requires 'Test2::V0';
};
```

---

## Perl Idioms & Patterns

### Pure Function Modules

For packages that are collections of functions (trig, matrix, logic-gates):

```perl
use v5.40;

package CodingAdventures::Trig;
use Exporter 'import';
our @EXPORT_OK = qw(sin_taylor cos_taylor tan_taylor PI deg_to_rad rad_to_deg);

use constant PI => 3.141592653589793;

# ---------------------------------------------------------------------------
# Range Reduction
# ---------------------------------------------------------------------------
#
# The Taylor series converges for any x, but converges *faster* when x is
# small. We normalize x into [-pi, pi] by subtracting multiples of 2*pi.
# This doesn't change the value of sin/cos because they're periodic.

sub _range_reduce($x) {
    my $TWO_PI = 2 * PI;
    $x -= $TWO_PI * int($x / $TWO_PI + 0.5);
    return $x;
}

sub sin_taylor($x, $terms = 20) {
    $x = _range_reduce($x);
    my $result = 0;
    my $power  = $x;
    my $fact   = 1;
    for my $n (0 .. $terms - 1) {
        $result += ((-1) ** $n) * $power / $fact;
        $power  *= $x * $x;
        $fact   *= (2 * $n + 2) * (2 * $n + 3);
    }
    return $result;
}
```

### Native Class Syntax (Perl 5.38+)

For packages that model objects (wave, cache, cpu-simulator):

```perl
use v5.40;
use feature 'class';

class CodingAdventures::Wave {
    field $amplitude  :param :reader;
    field $frequency  :param :reader;
    field $wavelength :param :reader;
    field $phase      :param :reader = 0;

    method period() {
        return 1.0 / $frequency;
    }

    method velocity() {
        return $frequency * $wavelength;
    }

    method displacement($x, $t) {
        my $angular_freq = 2 * CodingAdventures::Trig::PI * $frequency;
        my $wave_number  = 2 * CodingAdventures::Trig::PI / $wavelength;
        return $amplitude * CodingAdventures::Trig::sin_taylor(
            $wave_number * $x - $angular_freq * $t + $phase
        );
    }
}
```

### Tests with Test2::V0

```perl
use v5.40;
use Test2::V0;

use lib '../trig/lib';
use CodingAdventures::Trig qw(sin_taylor cos_taylor PI);

subtest 'sin(0) = 0' => sub {
    is(sin_taylor(0), 0, 'sin(0) should be 0');
};

subtest 'sin(pi/2) approximates 1' => sub {
    ok(abs(sin_taylor(PI / 2) - 1.0) < 1e-10,
       'sin(pi/2) should be 1 within 1e-10');
};

subtest 'Pythagorean identity' => sub {
    for my $angle (0, 0.5, 1.0, 1.5, 2.0, PI, PI / 4) {
        my $s = sin_taylor($angle);
        my $c = cos_taylor($angle);
        ok(abs($s**2 + $c**2 - 1.0) < 1e-10,
           "sin^2 + cos^2 = 1 for angle $angle");
    }
};

done_testing;
```

### Literate Programming with POD

All Perl modules use POD (Plain Old Documentation) interleaved with code:

```perl
=head1 NAME

CodingAdventures::Trig - Trigonometric functions from first principles

=head1 DESCRIPTION

This module implements sine and cosine using B<Taylor series> (specifically,
Maclaurin series -- Taylor series centered at zero). No math library is used;
everything is built from addition, multiplication, and division alone.

=head2 Why Taylor series?

Any "smooth" function can be approximated near a point by a polynomial. The
idea, due to Brook Taylor (1715), is:

    f(x) = f(0) + f'(0)*x + f''(0)*x^2/2! + f'''(0)*x^3/3! + ...

When centered at zero this is called a B<Maclaurin series>. For sine and
cosine the derivatives cycle through a simple pattern, giving us concrete
formulas we can compute with just arithmetic.

=cut

# Implementation follows...
```

Additionally, inline `#` comments explain logic step-by-step, with truth
tables, ASCII diagrams, and worked examples — matching the literate style
used in all other languages.

---

## Build Tool Integration

### Files to Modify

1. **`code/programs/go/build-tool/main.go`**
   - Add `"perl"` to the `allLanguages` slice

2. **`code/programs/go/build-tool/detect_languages_test.go`**
   - Add `"perl": true` to the expected languages map

3. **`code/programs/go/build-tool/internal/resolver/resolver.go`**
   - Add `case "perl":` to `buildKnownNames()`:
     ```go
     case "perl":
         cpanName := "codingadventures-" + strings.ReplaceAll(
             strings.ToLower(filepath.Base(pkg.Path)), "-", "")
         known[cpanName] = pkg.Name
     ```
   - Add `case "perl":` to the dependency parsing switch
   - Add `parsePerlDeps()` function to parse `cpanfile` for
     `requires 'CodingAdventures::...'` lines

4. **`code/programs/go/build-tool/internal/resolver/resolver_test.go`**
   - Add Perl test cases for name mapping and dependency parsing

5. **`.github/workflows/build.yml`** (or equivalent)
   - Add Perl setup: `shogo82148/actions-setup-perl@v1` with `perl-version: '5.40'`
   - Install test deps: `cpanm Test2::V0 Devel::Cover`

6. **`mise.toml`**
   - Add `perl = "5.40"`

---

## Porting Order

Packages are ordered by dependency layer — leaf packages first, then packages
that depend on them. Within each phase, packages can be ported in any order
(they are independent of each other).

### Phase 0: Infrastructure Setup
- Install Perl 5.40+ via mise
- Install tooling (cpanm, Test2::V0, Devel::Cover, Perl::Critic, Perl::Tidy)
- Update build tool for Perl support
- Create shared `.perlcriticrc` and `.perltidyrc`

### Phase 1: Standalone Leaf Packages (11 packages)

| # | Package | Module | Description |
|---|---------|--------|-------------|
| 1.1 | trig | CodingAdventures::Trig | Taylor series sin/cos/tan |
| 1.2 | wave | CodingAdventures::Wave | Wave physics (depends on trig) |
| 1.3 | progress-bar | CodingAdventures::ProgressBar | Terminal progress display |
| 1.4 | clock | CodingAdventures::Clock | Cycle-accurate clock |
| 1.5 | display | CodingAdventures::Display | Output utilities |
| 1.6 | tree | CodingAdventures::Tree | AST node representation |
| 1.7 | grammar-tools | CodingAdventures::GrammarTools | Grammar file utilities |
| 1.8 | fp-arithmetic | CodingAdventures::FPArithmetic | IEEE 754 floats |
| 1.9 | matrix | CodingAdventures::Matrix | Linear algebra |
| 1.10 | directed-graph | CodingAdventures::DirectedGraph | Graph + topological sort |
| 1.11 | brainfuck | CodingAdventures::Brainfuck | Brainfuck interpreter |

### Phase 2: Core Infrastructure (6 packages)

| # | Package | Module | Deps |
|---|---------|--------|------|
| 2.1 | state-machine | CodingAdventures::StateMachine | directed-graph |
| 2.2 | logic-gates | CodingAdventures::LogicGates | (none) |
| 2.3 | transistors | CodingAdventures::Transistors | (none) |
| 2.4 | activation-functions | CodingAdventures::ActivationFunctions | (none) |
| 2.5 | loss-functions | CodingAdventures::LossFunctions | (none) |
| 2.6 | gradient-descent | CodingAdventures::GradientDescent | (none) |

### Phase 3: Hardware Stack (9 packages)

| # | Package | Module | Deps |
|---|---------|--------|------|
| 3.1 | arithmetic | CodingAdventures::Arithmetic | logic-gates |
| 3.2 | block-ram | CodingAdventures::BlockRAM | logic-gates |
| 3.3 | cpu-simulator | CodingAdventures::CPUSimulator | arithmetic, logic-gates |
| 3.4 | branch-predictor | CodingAdventures::BranchPredictor | state-machine |
| 3.5 | hazard-detection | CodingAdventures::HazardDetection | (none) |
| 3.6 | cache | CodingAdventures::Cache | clock |
| 3.7 | cpu-pipeline | CodingAdventures::CPUPipeline | (none) |
| 3.8 | core | CodingAdventures::Core | cache, branch-predictor, hazard-detection, cpu-pipeline |
| 3.9 | fpga | CodingAdventures::FPGA | logic-gates, block-ram |

### Phase 4: Compiler Infrastructure (6 packages)

| # | Package | Module | Deps |
|---|---------|--------|------|
| 4.1 | lexer | CodingAdventures::Lexer | grammar-tools |
| 4.2 | parser | CodingAdventures::Parser | lexer, tree |
| 4.3 | virtual-machine | CodingAdventures::VirtualMachine | (none) |
| 4.4 | bytecode-compiler | CodingAdventures::BytecodeCompiler | tree, virtual-machine |
| 4.5 | assembler | CodingAdventures::Assembler | cpu-simulator |
| 4.6 | cli-builder | CodingAdventures::CLIBuilder | directed-graph, state-machine |

### Phase 5: Language Lexers & Parsers (17 packages)

All depend on `lexer` and/or `parser`:
- json-lexer, json-parser
- toml-lexer, toml-parser
- css-lexer, css-parser
- xml-lexer
- python-lexer, python-parser
- ruby-lexer, ruby-parser
- javascript-lexer, javascript-parser
- typescript-lexer, typescript-parser
- starlark-lexer, starlark-parser

### Phase 6: ISA Simulators (7 packages)

| # | Package | Module | Deps |
|---|---------|--------|------|
| 6.1 | intel4004-simulator | CodingAdventures::Intel4004Simulator | virtual-machine |
| 6.2 | jvm-simulator | CodingAdventures::JVMSimulator | virtual-machine |
| 6.3 | clr-simulator | CodingAdventures::CLRSimulator | virtual-machine |
| 6.4 | riscv-simulator | CodingAdventures::RISCVSimulator | cpu-simulator |
| 6.5 | arm-simulator | CodingAdventures::ARMSimulator | cpu-simulator |
| 6.6 | wasm-simulator | CodingAdventures::WASMSimulator | virtual-machine |
| 6.7 | intel4004-gatelevel | CodingAdventures::Intel4004GateLevel | logic-gates, arithmetic |

### Phase 7: ML Stack (6 packages)

| # | Package | Module | Deps |
|---|---------|--------|------|
| 7.1 | perceptron | CodingAdventures::Perceptron | activation-functions |
| 7.2 | blas-library | CodingAdventures::BLAS | matrix |
| 7.3 | ml-framework-core | CodingAdventures::MLFramework::Core | matrix, activation-functions, loss-functions, gradient-descent |
| 7.4 | ml-framework-keras | CodingAdventures::MLFramework::Keras | ml-framework-core |
| 7.5 | ml-framework-tf | CodingAdventures::MLFramework::TF | ml-framework-core, ml-framework-keras |
| 7.6 | ml-framework-torch | CodingAdventures::MLFramework::Torch | ml-framework-core |

### Phase 8: GPU/Accelerator Stack (6 packages)

gpu-core, parallel-execution-engine, compute-unit, device-simulator,
compute-runtime, vendor-api-simulators

### Phase 9: System Software Stack (11 packages)

interrupt-handler, rom-bios, bootloader, os-kernel, device-driver-framework,
virtual-memory, process-manager, file-system, ipc, network-stack, system-board

### Phase 10: Lisp & Starlark Compilers (8 packages)

lisp-lexer, lisp-parser, lisp-vm, lisp-compiler, starlark-compiler,
starlark-interpreter, starlark-vm, starlark-ast-to-bytecode-compiler

### Phase 11: Garbage Collector & JIT (2 packages)

garbage-collector, jit-compiler

### Phase 12: Programs (7+ programs)

build-tool, scaffold-generator, unix-tools, celsius-to-fahrenheit-predictor,
house-price-predictor, mansion-classifier, space-launch-predictor

---

## Verification Checklist

For each package:
- [ ] `prove -l t/` passes all tests
- [ ] `perl -c lib/CodingAdventures/PackageName.pm` compiles cleanly
- [ ] Coverage via Devel::Cover exceeds 80% (95%+ for libraries)
- [ ] `perlcritic lib/` passes at severity 3+
- [ ] BUILD file works from clean state
- [ ] Build tool discovers the package
- [ ] README.md documents purpose, usage, and examples
- [ ] CHANGELOG.md records initial release
- [ ] Literate programming style with POD and inline comments

For the overall port:
- [ ] `./build-tool --force` builds all Perl packages
- [ ] CI workflow includes Perl in the language matrix
- [ ] All ~96 packages + 7 programs ported and passing

---

## Package Count

| Phase | Count | Description |
|-------|-------|-------------|
| 0 | 0 | Infrastructure setup |
| 1 | 11 | Standalone leaf packages |
| 2 | 6 | Core infrastructure |
| 3 | 9 | Hardware stack |
| 4 | 6 | Compiler infrastructure |
| 5 | 17 | Language lexers/parsers |
| 6 | 7 | ISA simulators |
| 7 | 6 | ML stack |
| 8 | 6 | GPU/accelerator |
| 9 | 11 | System software |
| 10 | 8 | Lisp & Starlark compilers |
| 11 | 2 | GC & JIT |
| 12 | 7+ | Programs |
| **Total** | **~96 packages + 7 programs** | |
