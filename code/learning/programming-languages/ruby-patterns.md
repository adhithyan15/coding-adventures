# Ruby Patterns Used in This Project

This document explains the Ruby language features and patterns used
throughout the coding-adventures Ruby gems.

## Data.define — Immutable Value Objects (Ruby 3.2+)

Ruby 3.2 introduced `Data.define` for creating immutable value objects.
It's the Ruby equivalent of Python's `@dataclass(frozen=True)`:

```ruby
# Define an immutable record type
Package = Data.define(:name, :path, :build_commands, :language)

# Create an instance — all fields must be provided
pkg = Package.new(
  name: "python/logic-gates",
  path: Pathname("/repo/code/packages/python/logic-gates"),
  build_commands: ["pytest"],
  language: "python"
)

pkg.name        # => "python/logic-gates"
pkg.frozen?     # => true (immutable!)
pkg.name = "x"  # => FrozenError!
```

`Data.define` gives you:
- Immutable instances (frozen by default)
- Value equality (`==` compares fields, not identity)
- Pattern matching support
- Readable `#inspect` output

**Where used:** `code/packages/ruby/*/lib/` — for tokens, AST nodes, cache entries

## frozen_string_literal Pragma

Every Ruby file starts with:

```ruby
# frozen_string_literal: true
```

This freezes all string literals in the file, preventing accidental
mutation and enabling string deduplication:

```ruby
# frozen_string_literal: true

name = "hello"
name << " world"  # => FrozenError!
name = name + " world"  # OK — creates a new string
```

This is a performance optimization and safety measure. Standard Ruby
(the linter) requires it.

**Where used:** Every Ruby file

## Modules for Namespacing

Ruby uses nested modules for namespacing (like Python packages):

```ruby
module CodingAdventures
  module LogicGates
    VERSION = "0.1.0"

    def self.and_gate(a, b)
      a & b
    end
  end
end

CodingAdventures::LogicGates.and_gate(1, 1)  # => 1
```

The `module_function` keyword makes methods callable both as module
methods and as instance methods:

```ruby
module BuildTool
  module Discovery
    module_function

    def discover_packages(root)
      # Can call as Discovery.discover_packages(root)
      # or include Discovery and call discover_packages(root)
    end
  end
end
```

**Where used:** Every Ruby package

## Minitest — Simple, Fast Testing

Ruby's built-in test framework. Tests are methods starting with `test_`:

```ruby
class TestHalfAdder < Minitest::Test
  def test_zero_plus_zero
    sum, carry = CodingAdventures::Arithmetic.half_adder(0, 0)
    assert_equal 0, sum
    assert_equal 0, carry
  end

  def test_one_plus_one
    sum, carry = CodingAdventures::Arithmetic.half_adder(1, 1)
    assert_equal 0, sum   # sum bit
    assert_equal 1, carry # carry bit (1 + 1 = 10 in binary)
  end
end
```

Key assertions:
- `assert_equal expected, actual` — equality check
- `assert predicate` — truthiness
- `refute predicate` — falsiness
- `assert_raises(ErrorClass) { ... }` — exception check
- `assert_match /regex/, string` — pattern match
- `assert_in_delta expected, actual, tolerance` — floating point

**Where used:** Every Ruby package's `test/` directory

## SimpleCov — Coverage Measurement

Every test suite starts SimpleCov before loading application code:

```ruby
require "simplecov"
SimpleCov.start do
  enable_coverage :branch   # track branch coverage, not just line
  minimum_coverage 80       # fail if coverage drops below 80%
end

require "minitest/autorun"
require "coding_adventures_logic_gates"
```

**Critical:** SimpleCov must be started BEFORE requiring any application
code, or that code won't be tracked.

**Where used:** Every `test/test_helper.rb`

## Blocks, Procs, and Lambdas

Ruby blocks are anonymous functions passed to methods:

```ruby
# Block with do...end
[1, 0, 1, 0].each do |bit|
  puts bit
end

# Block with braces (single line)
[1, 0, 1, 0].select { |bit| bit == 1 }  # => [1, 1]

# Lambda (stored in a variable)
listener = ->(edge) { received << edge }
clock.register_listener(listener)
```

Common block-based patterns in the codebase:
- `.map(&:name)` — shorthand for `.map { |x| x.name }`
- `.reject { |line| line.empty? }` — filter out empties
- `.sort_by(&:name)` — sort by a field
- `.each_with_index` — iterate with index
- `FileList["test/**/test_*.rb"]` — glob pattern matching

**Where used:** Everywhere — blocks are fundamental Ruby

## Pathname — Object-Oriented File Paths

Like Python's `pathlib.Path`:

```ruby
require "pathname"

dir = Pathname("/repo/code/packages/ruby")
build_file = dir / "BUILD"          # join with /
build_file.exist?                    # check existence
build_file.read                      # read contents
dir.children.select(&:directory?)    # list subdirectories
dir.basename                         # last component
```

**Where used:** `code/programs/ruby/build-tool/`

## Gemspec — Package Metadata

Every Ruby package has a `.gemspec` file defining its metadata:

```ruby
Gem::Specification.new do |spec|
  spec.name = "coding_adventures_arithmetic"
  spec.version = CodingAdventures::Arithmetic::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Arithmetic circuits — Layer 9"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"

  spec.files = Dir["lib/**/*.rb", "sig/**/*.rbs", "README.md"]
  spec.require_paths = ["lib"]

  spec.add_dependency "coding_adventures_logic_gates", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
```

Key conventions:
- Ruby gems use underscores in names: `coding_adventures_arithmetic`
- Development dependencies are for testing/linting only
- `~> 0.1` means "compatible with 0.1.x" (pessimistic version constraint)
- `spec.metadata["rubygems_mfa_required"] = "true"` enables 2FA for publishing

**Where used:** Every Ruby package

## RBS Type Signatures

Ruby's official type annotation system uses `.rbs` files:

```ruby
# sig/coding_adventures/arithmetic/adders.rbs
module CodingAdventures
  module Arithmetic
    def self.half_adder: (Integer, Integer) -> [Integer, Integer]
    def self.full_adder: (Integer, Integer, Integer) -> [Integer, Integer]
  end
end
```

RBS files live in `sig/` and are separate from the source code (unlike
Python's inline type hints). They can be checked by Steep or Sorbet.

**Where used:** Several Ruby packages have `sig/` directories

## Rakefile — Task Runner

Every Ruby package has a Rakefile:

```ruby
require "rake/testtask"

Rake::TestTask.new(:test) do |t|
  t.libs << "test"
  t.libs << "lib"
  t.test_files = FileList["test/**/test_*.rb"]
end

task default: :test
```

`bundle exec rake test` runs all tests. `bundle exec rake` runs the
default task (which is `:test`).

**Where used:** Every Ruby package
