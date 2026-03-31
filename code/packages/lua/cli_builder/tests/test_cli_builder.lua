-- Tests for cli_builder
-- =======================
--
-- Comprehensive busted test suite for the CLI Builder package.
--
-- Coverage:
--   - Module loads and exposes the public API
--   - TokenClassifier: all token kinds
--   - SpecLoader: valid spec, missing required fields, invalid type
--   - HelpGenerator: basic help text
--   - Parser: flags, subcommands, positionals, defaults, help, version, errors

package.path = (
    "../src/?.lua;"  ..
    "../src/?/init.lua;"  ..
    package.path
)

local cli = require("coding_adventures.cli_builder")

-- ============================================================================
-- Test spec helpers
-- ============================================================================

local ECHO_SPEC = {
  cli_builder_spec_version = "1.0",
  name        = "echo",
  description = "Print text to stdout",
  version     = "1.0.0",
  builtin_flags = { help = true, version = true },
  flags = {
    {
      id = "newline", short = "n", long = "newline",
      description = "Suppress newline", type = "boolean",
    },
    {
      id = "count", short = "c", long = "count",
      description = "Repeat count", type = "integer",
      default = 1,
    },
  },
  arguments = {
    { id = "string", display_name = "STRING", description = "Text to print",
      required = true, variadic = true },
  },
  commands = {},
}

local GIT_SPEC = {
  cli_builder_spec_version = "1.0",
  name        = "git",
  description = "The git version control system",
  version     = "2.40.0",
  builtin_flags = { help = true, version = true },
  global_flags = {
    { id = "verbose", short = "v", long = "verbose", description = "Verbose output", type = "boolean" },
  },
  flags = {},
  arguments = {},
  commands = {
    {
      name = "commit",
      description = "Record changes to the repository",
      flags = {
        { id = "message", short = "m", long = "message",
          description = "Commit message", type = "string", required = true },
        { id = "amend", long = "amend", description = "Amend last commit", type = "boolean" },
      },
      arguments = {},
      commands = {},
    },
    {
      name = "remote",
      description = "Manage remote repositories",
      flags = {},
      arguments = {},
      commands = {
        {
          name = "add",
          description = "Add a remote",
          flags = {},
          arguments = {
            { id = "name", display_name = "NAME", description = "Remote name", required = true },
            { id = "url",  display_name = "URL",  description = "Remote URL",  required = true },
          },
          commands = {},
        },
      },
    },
  },
}

local ENUM_SPEC = {
  cli_builder_spec_version = "1.0",
  name = "format",
  description = "Format converter",
  flags = {
    { id = "output", short = "o", long = "output",
      description = "Output format", type = "enum",
      enum_values = { "json", "csv", "xml" },
    },
  },
  arguments = {},
  commands = {},
}

-- ============================================================================
-- Module API
-- ============================================================================

describe("cli_builder module", function()
  it("loads successfully", function()
    assert.is_not_nil(cli)
  end)

  it("exposes TokenClassifier", function()
    assert.is_not_nil(cli.TokenClassifier)
  end)

  it("exposes SpecLoader", function()
    assert.is_not_nil(cli.SpecLoader)
  end)

  it("exposes HelpGenerator", function()
    assert.is_not_nil(cli.HelpGenerator)
  end)

  it("exposes Parser", function()
    assert.is_not_nil(cli.Parser)
  end)

  it("exposes parse_table", function()
    assert.is_function(cli.parse_table)
  end)
end)

-- ============================================================================
-- TokenClassifier
-- ============================================================================

describe("TokenClassifier", function()
  local flags = {
    { id="verbose", short="v", long="verbose", single_dash_long=nil, type="boolean" },
    { id="output",  short="o", long="output",  single_dash_long=nil, type="string"  },
    { id="count",   short="c", long="count",   single_dash_long=nil, type="count"   },
    { id="cp",      short=nil, long=nil,        single_dash_long="classpath", type="string" },
  }

  it("-- is end_of_flags", function()
    local t = cli.TokenClassifier.classify("--", flags)
    assert.equal("end_of_flags", t.kind)
  end)

  it("--verbose is long_flag", function()
    local t = cli.TokenClassifier.classify("--verbose", flags)
    assert.equal("long_flag", t.kind)
    assert.equal("verbose", t.name)
  end)

  it("--output=foo is long_flag_value", function()
    local t = cli.TokenClassifier.classify("--output=foo.txt", flags)
    assert.equal("long_flag_value", t.kind)
    assert.equal("output", t.name)
    assert.equal("foo.txt", t.value)
  end)

  it("-v is short_flag", function()
    local t = cli.TokenClassifier.classify("-v", flags)
    assert.equal("short_flag", t.kind)
    assert.equal("v", t.char)
  end)

  it("-ofile is short_flag_value for non-boolean", function()
    local t = cli.TokenClassifier.classify("-ofile.txt", flags)
    assert.equal("short_flag_value", t.kind)
    assert.equal("o", t.char)
    assert.equal("file.txt", t.value)
  end)

  it("-vc is stacked_flags (boolean+count)", function()
    local t = cli.TokenClassifier.classify("-vc", flags)
    assert.equal("stacked_flags", t.kind)
    assert.is_table(t.chars)
  end)

  it("hello is positional", function()
    local t = cli.TokenClassifier.classify("hello", flags)
    assert.equal("positional", t.kind)
    assert.equal("hello", t.value)
  end)

  it("- is positional", function()
    local t = cli.TokenClassifier.classify("-", flags)
    assert.equal("positional", t.kind)
  end)

  it("-classpath is single_dash_long", function()
    local t = cli.TokenClassifier.classify("-classpath", flags)
    assert.equal("single_dash_long", t.kind)
    assert.equal("classpath", t.name)
  end)

  it("--unknown is long_flag with unknown name (classified then rejected by parser)", function()
    local t = cli.TokenClassifier.classify("--unknown-flag", flags)
    assert.equal("long_flag", t.kind)
    assert.equal("unknown-flag", t.name)
  end)

  it("-z is unknown_flag", function()
    local t = cli.TokenClassifier.classify("-z", flags)
    assert.equal("unknown_flag", t.kind)
  end)
end)

-- ============================================================================
-- SpecLoader
-- ============================================================================

describe("SpecLoader", function()
  it("loads valid spec table", function()
    local spec = cli.SpecLoader.load_table(ECHO_SPEC)
    assert.is_table(spec)
    assert.equal("echo", spec.name)
    assert.equal("1.0.0", spec.version)
  end)

  it("normalizes flags", function()
    local spec = cli.SpecLoader.load_table(ECHO_SPEC)
    assert.equal(2, #spec.flags)
    local f = spec.flags[1]
    assert.equal("newline", f.id)
    assert.equal("boolean", f.type)
    assert.is_boolean(f.required)
    assert.is_table(f.conflicts_with)
    assert.is_table(f.requires)
  end)

  it("normalizes arguments", function()
    local spec = cli.SpecLoader.load_table(ECHO_SPEC)
    assert.equal(1, #spec.arguments)
    assert.equal("string", spec.arguments[1].id)
    assert.is_true(spec.arguments[1].variadic)
  end)

  it("applies default parsing_mode=gnu", function()
    local spec = cli.SpecLoader.load_table(ECHO_SPEC)
    assert.equal("gnu", spec.parsing_mode)
  end)

  it("raises on missing name", function()
    assert.has_error(function()
      cli.SpecLoader.load_table({
        cli_builder_spec_version = "1.0",
        description = "test"
      })
    end)
  end)

  it("raises on invalid spec version", function()
    assert.has_error(function()
      cli.SpecLoader.load_table({
        cli_builder_spec_version = "99.0",
        name = "test", description = "test"
      })
    end)
  end)

  it("raises on flag missing id", function()
    assert.has_error(function()
      cli.SpecLoader.load_table({
        cli_builder_spec_version = "1.0",
        name = "test", description = "test",
        flags = { { description = "x", type = "boolean" } }
      })
    end)
  end)

  it("raises on invalid flag type", function()
    assert.has_error(function()
      cli.SpecLoader.load_table({
        cli_builder_spec_version = "1.0",
        name = "test", description = "test",
        flags = { { id = "x", description = "y", type = "invalid_type" } }
      })
    end)
  end)

  it("raises on enum without enum_values", function()
    assert.has_error(function()
      cli.SpecLoader.load_table({
        cli_builder_spec_version = "1.0",
        name = "test", description = "test",
        flags = { { id = "x", description = "y", type = "enum" } }
      })
    end)
  end)
end)

-- ============================================================================
-- HelpGenerator
-- ============================================================================

describe("HelpGenerator", function()
  local spec

  before_each(function()
    spec = cli.SpecLoader.load_table(ECHO_SPEC)
  end)

  it("generates a string", function()
    local text = cli.HelpGenerator.generate(spec, {"echo"})
    assert.is_string(text)
    assert.truthy(#text > 0)
  end)

  it("includes USAGE section", function()
    local text = cli.HelpGenerator.generate(spec, {"echo"})
    assert.truthy(text:find("USAGE"))
  end)

  it("includes program name in usage", function()
    local text = cli.HelpGenerator.generate(spec, {"echo"})
    assert.truthy(text:find("echo"))
  end)

  it("includes DESCRIPTION section", function()
    local text = cli.HelpGenerator.generate(spec, {"echo"})
    assert.truthy(text:find("DESCRIPTION"))
  end)

  it("includes OPTIONS for flags", function()
    local text = cli.HelpGenerator.generate(spec, {"echo"})
    assert.truthy(text:find("OPTIONS") or text:find("newline") or text:find("count"))
  end)

  it("generates help for subcommand", function()
    local git_spec = cli.SpecLoader.load_table(GIT_SPEC)
    local text = cli.HelpGenerator.generate(git_spec, {"git", "commit"})
    assert.is_string(text)
    assert.truthy(text:find("commit") or text:find("message"))
  end)
end)

-- ============================================================================
-- Parser — basic flag parsing
-- ============================================================================

describe("Parser basic flags", function()
  it("boolean flag --newline sets to true", function()
    local r = cli.parse_table(ECHO_SPEC, {"--newline", "hello"})
    assert.equal("result", r.type)
    assert.is_true(r.flags["newline"])
  end)

  it("short boolean flag -n sets to true", function()
    local r = cli.parse_table(ECHO_SPEC, {"-n", "hello"})
    assert.equal("result", r.type)
    assert.is_true(r.flags["newline"])
  end)

  it("integer flag --count=3 is coerced to 3", function()
    local r = cli.parse_table(ECHO_SPEC, {"--count=3", "hello"})
    assert.equal("result", r.type)
    assert.equal(3, r.flags["count"])
  end)

  it("short flag -c 5 takes next token as value", function()
    local r = cli.parse_table(ECHO_SPEC, {"-c", "5", "hello"})
    assert.equal("result", r.type)
    assert.equal(5, r.flags["count"])
  end)

  it("absent boolean flag defaults to false", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    assert.equal("result", r.type)
    assert.is_false(r.flags["newline"])
  end)

  it("absent integer flag uses spec default", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    assert.equal("result", r.type)
    assert.equal(1, r.flags["count"])
  end)

  it("tokens after -- are positional", function()
    local r = cli.parse_table(ECHO_SPEC, {"--", "--not-a-flag"})
    assert.equal("result", r.type)
    -- --not-a-flag should be treated as positional
    assert.truthy(r.arguments["string"] ~= nil)
  end)
end)

-- ============================================================================
-- Parser — positional arguments
-- ============================================================================

describe("Parser positional arguments", function()
  it("variadic argument collects all positionals", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello", "world"})
    assert.equal("result", r.type)
    assert.is_table(r.arguments["string"])
    assert.equal(2, #r.arguments["string"])
    assert.equal("hello", r.arguments["string"][1])
    assert.equal("world", r.arguments["string"][2])
  end)

  it("required variadic argument error when missing", function()
    local r = cli.parse_table(ECHO_SPEC, {})
    assert.equal("error", r.type)
    -- Should have a missing_argument error
    local has_error = false
    for _, e in ipairs(r.errors) do
      if e.error_type == "missing_argument" then has_error = true; break end
    end
    assert.is_true(has_error)
  end)
end)

-- ============================================================================
-- Parser — subcommands
-- ============================================================================

describe("Parser subcommands", function()
  it("routes to commit subcommand", function()
    local r = cli.parse_table(GIT_SPEC, {"commit", "-m", "Initial commit"})
    assert.equal("result", r.type)
    assert.is_table(r.command_path)
    assert.equal("commit", r.command_path[2])
    assert.equal("Initial commit", r.flags["message"])
  end)

  it("routes to nested subcommand remote add", function()
    local r = cli.parse_table(GIT_SPEC, {"remote", "add", "origin", "https://github.com/user/repo"})
    assert.equal("result", r.type)
    assert.equal("add", r.command_path[3])
    assert.equal("origin", r.arguments["name"])
    assert.equal("https://github.com/user/repo", r.arguments["url"])
  end)

  it("global flags work in subcommand context", function()
    local r = cli.parse_table(GIT_SPEC, {"--verbose", "commit", "-m", "msg"})
    assert.equal("result", r.type)
    assert.is_true(r.flags["verbose"])
  end)

  it("required flag error in subcommand", function()
    local r = cli.parse_table(GIT_SPEC, {"commit"})
    assert.equal("error", r.type)
    -- "message" flag is required
    local has_error = false
    for _, e in ipairs(r.errors) do
      if e.error_type == "flag_error" and e.message:find("message") then
        has_error = true; break
      end
    end
    assert.is_true(has_error)
  end)
end)

-- ============================================================================
-- Parser — help and version
-- ============================================================================

describe("Parser help and version", function()
  it("--help returns help result", function()
    local r = cli.parse_table(ECHO_SPEC, {"--help"})
    assert.equal("help", r.type)
    assert.is_string(r.text)
    assert.is_table(r.command_path)
  end)

  it("-h returns help result", function()
    local r = cli.parse_table(ECHO_SPEC, {"-h"})
    assert.equal("help", r.type)
  end)

  it("--version returns version result", function()
    local r = cli.parse_table(ECHO_SPEC, {"--version"})
    assert.equal("version", r.type)
    assert.equal("1.0.0", r.version)
  end)

  it("help text includes program name", function()
    local r = cli.parse_table(ECHO_SPEC, {"--help"})
    assert.truthy(r.text:find("echo"))
  end)
end)

-- ============================================================================
-- Parser — enum validation
-- ============================================================================

describe("Parser enum validation", function()
  it("valid enum value accepted", function()
    local r = cli.parse_table(ENUM_SPEC, {"--output=json"})
    assert.equal("result", r.type)
    assert.equal("json", r.flags["output"])
  end)

  it("invalid enum value causes error", function()
    local r = cli.parse_table(ENUM_SPEC, {"--output=pdf"})
    assert.equal("error", r.type)
    local has_error = false
    for _, e in ipairs(r.errors) do
      if e.error_type == "flag_error" and e.message:find("invalid value") then
        has_error = true; break
      end
    end
    assert.is_true(has_error)
  end)
end)

-- ============================================================================
-- Parser — unknown flags
-- ============================================================================

describe("Parser unknown flags", function()
  it("unknown long flag causes error", function()
    local r = cli.parse_table(ECHO_SPEC, {"--unknown-flag"})
    assert.equal("error", r.type)
    local has_error = false
    for _, e in ipairs(r.errors) do
      if e.error_type == "unknown_flag" then has_error = true; break end
    end
    assert.is_true(has_error)
  end)

  it("unknown short flag causes error", function()
    local r = cli.parse_table(ECHO_SPEC, {"-z"})
    assert.equal("error", r.type)
  end)
end)

-- ============================================================================
-- Parser — flag with missing value
-- ============================================================================

describe("Parser flag missing value", function()
  it("string flag at end of argv causes error", function()
    local r = cli.parse_table(ECHO_SPEC, {"--count"})
    assert.equal("error", r.type)
    local has_error = false
    for _, e in ipairs(r.errors) do
      if e.error_type == "missing_flag_value" then has_error = true; break end
    end
    assert.is_true(has_error)
  end)
end)

-- ============================================================================
-- Parser — result structure
-- ============================================================================

describe("Parser result structure", function()
  it("result has program, command_path, flags, arguments, explicit_flags", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    assert.equal("result", r.type)
    assert.is_string(r.program)
    assert.is_table(r.command_path)
    assert.is_table(r.flags)
    assert.is_table(r.arguments)
    assert.is_table(r.explicit_flags)
  end)

  it("program matches spec name", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    assert.equal("echo", r.program)
  end)

  it("command_path starts with program name", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    assert.equal("echo", r.command_path[1])
  end)

  it("explicit_flags lists explicitly set flags", function()
    local r = cli.parse_table(ECHO_SPEC, {"--newline", "hello"})
    assert.equal("result", r.type)
    local found = false
    for _, id in ipairs(r.explicit_flags) do
      if id == "newline" then found = true; break end
    end
    assert.is_true(found)
  end)

  it("explicit_flags does not include defaulted flags", function()
    local r = cli.parse_table(ECHO_SPEC, {"hello"})
    -- "newline" was not explicitly set
    local found = false
    for _, id in ipairs(r.explicit_flags) do
      if id == "newline" then found = true; break end
    end
    assert.is_false(found)
  end)
end)
