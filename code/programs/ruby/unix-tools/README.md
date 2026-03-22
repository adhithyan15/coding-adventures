# unix-tools -- POSIX Utilities Powered by CLI Builder

A collection of reimplemented POSIX utilities, each powered by [CLI Builder](../../../packages/ruby/cli_builder/). Every tool's command-line interface -- flags, help text, version output, error messages -- is defined declaratively in a JSON spec. The Ruby source files contain only business logic.

## Tools

### pwd

Print the absolute pathname of the current working directory.

```bash
# Print logical working directory (default)
ruby pwd_tool.rb

# Print physical working directory (resolve symlinks)
ruby pwd_tool.rb -P

# Explicitly request logical path
ruby pwd_tool.rb -L

# Show help
ruby pwd_tool.rb --help
```

| Flag | Long Form | Description |
|------|-----------|-------------|
| `-L` | `--logical` | Display the logical current working directory (default) |
| `-P` | `--physical` | Display the physical current working directory (resolve all symlinks) |

## How It Works

```
tool.json (declarative spec)       tool.rb (business logic only)
+-------------------------+       +-----------------------------+
| flags, arguments        |       | Read spec result            |
| mutual exclusivity      |------>| Execute business logic      |
| help text, version      |       | Print output                |
| error messages          |       |                             |
+-------------------------+       +-----------------------------+
        CLI Builder                       Your code
     handles all of this              handles only this
```

## Where It Fits in the Stack

```
Layer 8: CLI Builder (argument parsing, help, validation)
    +-- This package: unix-tools (business logic only)

Layer 4: State Machine (drives CLI Builder's parsing modes)
Layer 3: Directed Graph (drives CLI Builder's command routing)
```

## Testing

```bash
bundle exec rake test
```
