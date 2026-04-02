# cli_builder (Lua)

Declarative CLI argument parser driven by JSON specifications.

## Usage

```lua
local cli = require("coding_adventures.cli_builder")

local spec = {
  cli_builder_spec_version = "1.0",
  name = "myapp",
  description = "My application",
  flags = {
    { id = "verbose", short = "v", long = "verbose",
      description = "Verbose output", type = "boolean" },
  },
  arguments = {
    { id = "file", display_name = "FILE",
      description = "Input file", required = true },
  },
  commands = {},
}

local result = cli.parse_table(spec, {"--verbose", "input.txt"})
if result.type == "result" then
  print(result.flags["verbose"])    -- true
  print(result.arguments["file"])   -- "input.txt"
elseif result.type == "help" then
  print(result.text)
elseif result.type == "error" then
  for _, e in ipairs(result.errors) do
    print(e.message)
  end
end
```

## Dependencies

No required dependencies. Optionally uses `dkjson`, `cjson`, or `json` for JSON loading.
