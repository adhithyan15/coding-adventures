# CA Capability Analyzer (Ruby)

Static capability analyzer for Ruby source code. Walks Ruby ASTs (via the Prism parser) to detect OS capability usage and banned dynamic execution constructs.

## What It Does

1. **Capability Detection** -- Scans Ruby source files for filesystem, network, process, environment, and FFI access patterns.
2. **Banned Construct Detection** -- Flags dynamic execution constructs (eval, send with dynamic args, backticks, method_missing, etc.) that evade static analysis.
3. **Manifest Comparison** -- Compares detected capabilities against a `required_capabilities.json` manifest for CI gating.

## Capability Categories

| Category | Examples |
|----------|----------|
| `fs`     | File.read, Dir.glob, FileUtils.rm |
| `net`    | TCPSocket.new, Net::HTTP.get, require "socket" |
| `proc`   | system(), exec(), Process.spawn, backticks |
| `env`    | ENV["KEY"], ENV.fetch, Dir.home |
| `ffi`    | require "fiddle", require "ffi" |

## Usage

```bash
# Detect capabilities in a file or directory
ca-capability-analyzer detect lib/

# Check against a manifest
ca-capability-analyzer check --manifest required_capabilities.json lib/

# Scan for banned constructs
ca-capability-analyzer banned lib/

# JSON output for CI
ca-capability-analyzer detect --json lib/
```

## Ruby API

```ruby
require "ca_capability_analyzer"

# Detect capabilities
caps = CA::CapabilityAnalyzer.analyze_file("lib/my_module.rb")
caps.each { |c| puts "#{c.file}:#{c.line}: #{c}" }

# Detect banned constructs
violations = CA::CapabilityAnalyzer.detect_banned("lib/my_module.rb")
violations.each { |v| puts v }

# Load manifest and compare
manifest = CA::CapabilityAnalyzer.load_manifest("required_capabilities.json")
result = CA::CapabilityAnalyzer.compare_capabilities(caps, manifest)
puts result.summary
```

## Development

```bash
bundle install
bundle exec rake test
bundle exec standardrb
```

## Dependencies

- **Runtime**: [prism](https://github.com/ruby/prism) (Ruby's official parser)
- **Dev**: minitest, simplecov, standard, rake
