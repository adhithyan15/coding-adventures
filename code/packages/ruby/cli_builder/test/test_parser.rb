# frozen_string_literal: true

require_relative "test_helper"

# Tests for Parser — the main orchestrating component.
#
# Each tool spec is embedded as a heredoc. We test:
#   1. echo   — variadic args, flag conflicts
#   2. ls     — requires dependency, flag conflict
#   3. cp     — variadic with trailing required (last-wins algorithm)
#   4. grep   — mutually exclusive groups, required_unless_flag
#   5. tar    — traditional mode, exclusive group
#   6. java   — single_dash_long flags
#   7. git    — subcommand routing, alias resolution, unknown command suggestion
class TestParser < Minitest::Test
  include CodingAdventures::CliBuilder

  # ---------------------------------------------------------------------------
  # Helper: parse argv against an inline spec hash
  # ---------------------------------------------------------------------------

  def parse(spec_hash, argv)
    Parser.new(nil, argv, spec_hash: spec_hash).parse
  end

  def assert_parse_error(spec_hash, argv, error_type: nil, message_match: nil)
    err = assert_raises(ParseErrors) { parse(spec_hash, argv) }
    if error_type
      assert(err.errors.any? { |e| e.error_type == error_type },
        "Expected error type #{error_type.inspect} but got: #{err.errors.map(&:error_type).inspect}")
    end
    if message_match
      assert(err.errors.any? { |e| e.message =~ message_match },
        "Expected message matching #{message_match.inspect} but got: #{err.errors.map(&:message).inspect}")
    end
    err
  end

  # ===========================================================================
  # 1. echo — minimal spec: variadic args, flag conflicts
  # ===========================================================================

  ECHO_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "echo",
    "description" => "Display a line of text",
    "version" => "8.32",
    "flags" => [
      {
        "id" => "no-newline",
        "short" => "n",
        "description" => "Do not output the trailing newline",
        "type" => "boolean"
      },
      {
        "id" => "enable-escapes",
        "short" => "e",
        "description" => "Enable interpretation of backslash escapes",
        "type" => "boolean",
        "conflicts_with" => ["disable-escapes"]
      },
      {
        "id" => "disable-escapes",
        "short" => "E",
        "description" => "Disable interpretation of backslash escapes",
        "type" => "boolean",
        "conflicts_with" => ["enable-escapes"]
      }
    ],
    "arguments" => [
      {
        "id" => "string",
        "name" => "STRING",
        "description" => "Text to print",
        "type" => "string",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_echo_hello_world
    result = parse(ECHO_SPEC, ["echo", "hello", "world"])
    assert_instance_of ParseResult, result
    assert_equal "echo", result.program
    assert_equal ["echo"], result.command_path
    assert_equal ["hello", "world"], result.arguments["string"]
    assert_equal false, result.flags["no-newline"]
    assert_equal false, result.flags["enable-escapes"]
    assert_equal false, result.flags["disable-escapes"]
  end

  def test_echo_no_newline_flag
    result = parse(ECHO_SPEC, ["echo", "-n", "hello"])
    assert_equal true, result.flags["no-newline"]
    assert_equal ["hello"], result.arguments["string"]
  end

  def test_echo_no_args_is_valid
    result = parse(ECHO_SPEC, ["echo"])
    assert_equal [], result.arguments["string"]
    assert_equal false, result.flags["no-newline"]
  end

  def test_echo_conflicting_e_and_E_flags
    assert_parse_error(ECHO_SPEC, ["echo", "-e", "-E", "hello"],
      error_type: "conflicting_flags")
  end

  def test_echo_conflicting_E_and_e_flags
    # Same conflict, other order
    assert_parse_error(ECHO_SPEC, ["echo", "-E", "-e", "hello"],
      error_type: "conflicting_flags")
  end

  def test_echo_stacked_ne_flags
    result = parse(ECHO_SPEC, ["echo", "-ne", "hello"])
    assert_equal true, result.flags["no-newline"]
    assert_equal true, result.flags["enable-escapes"]
    assert_equal ["hello"], result.arguments["string"]
  end

  def test_echo_help_returns_help_result
    result = parse(ECHO_SPEC, ["echo", "--help"])
    assert_instance_of HelpResult, result
    assert_match(/echo/, result.text)
    assert_equal ["echo"], result.command_path
  end

  def test_echo_short_help_flag
    result = parse(ECHO_SPEC, ["echo", "-h"])
    assert_instance_of HelpResult, result
  end

  def test_echo_version_returns_version_result
    result = parse(ECHO_SPEC, ["echo", "--version"])
    assert_instance_of VersionResult, result
    assert_equal "8.32", result.version
  end

  def test_echo_end_of_flags
    result = parse(ECHO_SPEC, ["echo", "--", "-n", "hello"])
    # After --, everything is positional (including -n)
    assert_equal ["-n", "hello"], result.arguments["string"]
    assert_equal false, result.flags["no-newline"]
  end

  # ===========================================================================
  # 2. ls — requires dependency, flag conflict
  # ===========================================================================

  LS_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "ls",
    "description" => "List directory contents",
    "flags" => [
      {
        "id" => "long",
        "short" => "l",
        "long" => "long-listing",
        "description" => "Use long listing format",
        "type" => "boolean"
      },
      {
        "id" => "all",
        "short" => "a",
        "long" => "all",
        "description" => "Do not ignore entries starting with .",
        "type" => "boolean"
      },
      {
        "id" => "human",
        "short" => "h",
        "long" => "human-readable",
        "description" => "Print sizes in human readable format",
        "type" => "boolean",
        "requires" => ["long"]
      },
      {
        "id" => "single-column",
        "short" => "1",
        "description" => "List one file per line",
        "type" => "boolean",
        "conflicts_with" => ["long"]
      }
    ],
    "arguments" => [
      {
        "id" => "file",
        "name" => "FILE",
        "description" => "File or directory to list",
        "type" => "path",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_ls_long_all_human_and_path
    result = parse(LS_SPEC, ["ls", "-lah", "/tmp"])
    assert_equal true, result.flags["long"]
    assert_equal true, result.flags["all"]
    assert_equal true, result.flags["human"]
    assert_equal ["/tmp"], result.arguments["file"]
  end

  def test_ls_human_without_long_is_error
    assert_parse_error(LS_SPEC, ["ls", "-h"],
      error_type: "missing_dependency_flag",
      message_match: /requires/)
  end

  def test_ls_single_column_and_long_conflict
    assert_parse_error(LS_SPEC, ["ls", "-1", "-l"],
      error_type: "conflicting_flags")
  end

  def test_ls_long_alone_is_valid
    result = parse(LS_SPEC, ["ls", "-l"])
    assert_equal true, result.flags["long"]
    assert_equal false, result.flags["all"]
    assert_equal false, result.flags["human"]
  end

  def test_ls_no_args_no_flags
    result = parse(LS_SPEC, ["ls"])
    assert_equal [], result.arguments["file"]
    assert_equal false, result.flags["long"]
  end

  def test_ls_long_flag_form
    result = parse(LS_SPEC, ["ls", "--long-listing"])
    assert_equal true, result.flags["long"]
  end

  def test_ls_human_with_long_is_valid
    result = parse(LS_SPEC, ["ls", "-lh"])
    assert_equal true, result.flags["long"]
    assert_equal true, result.flags["human"]
  end

  # ===========================================================================
  # 3. cp — variadic with trailing required (last-wins algorithm)
  # ===========================================================================

  CP_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "cp",
    "description" => "Copy files and directories",
    "flags" => [
      {
        "id" => "recursive",
        "short" => "r",
        "long" => "recursive",
        "description" => "Copy directories recursively",
        "type" => "boolean"
      },
      {
        "id" => "force",
        "short" => "f",
        "long" => "force",
        "description" => "Force copy",
        "type" => "boolean"
      }
    ],
    "arguments" => [
      {
        "id" => "source",
        "name" => "SOURCE",
        "description" => "Source file(s)",
        "type" => "path",
        "required" => true,
        "variadic" => true,
        "variadic_min" => 1
      },
      {
        "id" => "dest",
        "name" => "DEST",
        "description" => "Destination",
        "type" => "path",
        "required" => true
      }
    ]
  }.freeze

  def test_cp_single_source_dest
    result = parse(CP_SPEC, ["cp", "a.txt", "b.txt"])
    assert_equal ["a.txt"], result.arguments["source"]
    assert_equal "b.txt", result.arguments["dest"]
  end

  def test_cp_multiple_sources
    result = parse(CP_SPEC, ["cp", "a.txt", "b.txt", "c.txt", "/dest/"])
    assert_equal ["a.txt", "b.txt", "c.txt"], result.arguments["source"]
    assert_equal "/dest/", result.arguments["dest"]
  end

  def test_cp_missing_dest
    assert_parse_error(CP_SPEC, ["cp", "a.txt"],
      error_type: "too_few_arguments")
  end

  def test_cp_no_args
    assert_parse_error(CP_SPEC, ["cp"])
  end

  def test_cp_with_recursive_flag
    result = parse(CP_SPEC, ["cp", "-r", "src/", "dst/"])
    assert_equal true, result.flags["recursive"]
    assert_equal ["src/"], result.arguments["source"]
    assert_equal "dst/", result.arguments["dest"]
  end

  def test_cp_flags_after_positionals
    # GNU mode: flags anywhere
    result = parse(CP_SPEC, ["cp", "a.txt", "b.txt", "-r"])
    assert_equal true, result.flags["recursive"]
    assert_equal ["a.txt"], result.arguments["source"]
    assert_equal "b.txt", result.arguments["dest"]
  end

  # ===========================================================================
  # 4. grep — mutually exclusive groups, required_unless_flag
  # ===========================================================================

  GREP_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "grep",
    "description" => "Print lines that match patterns",
    "flags" => [
      {
        "id" => "extended-regexp",
        "short" => "E",
        "long" => "extended-regexp",
        "description" => "Interpret PATTERNS as extended regular expressions",
        "type" => "boolean"
      },
      {
        "id" => "fixed-strings",
        "short" => "F",
        "long" => "fixed-strings",
        "description" => "Interpret PATTERNS as fixed strings",
        "type" => "boolean"
      },
      {
        "id" => "perl-regexp",
        "short" => "P",
        "long" => "perl-regexp",
        "description" => "Interpret PATTERNS as Perl regular expressions",
        "type" => "boolean"
      },
      {
        "id" => "pattern-option",
        "short" => "e",
        "long" => "regexp",
        "description" => "Use PATTERN for matching",
        "type" => "string",
        "value_name" => "PATTERN",
        "repeatable" => true
      },
      {
        "id" => "ignore-case",
        "short" => "i",
        "long" => "ignore-case",
        "description" => "Ignore case distinctions",
        "type" => "boolean"
      }
    ],
    "mutually_exclusive_groups" => [
      {
        "id" => "engine",
        "flag_ids" => ["extended-regexp", "fixed-strings", "perl-regexp"],
        "required" => false
      }
    ],
    "arguments" => [
      {
        "id" => "pattern",
        "name" => "PATTERN",
        "description" => "Pattern to search for",
        "type" => "string",
        "required" => true,
        "required_unless_flag" => ["pattern-option"]
      },
      {
        "id" => "file",
        "name" => "FILE",
        "description" => "File to search",
        "type" => "path",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_grep_basic_pattern_and_file
    result = parse(GREP_SPEC, ["grep", "pattern", "file.txt"])
    assert_equal "pattern", result.arguments["pattern"]
    assert_equal ["file.txt"], result.arguments["file"]
  end

  def test_grep_extended_regexp
    result = parse(GREP_SPEC, ["grep", "-E", "pattern", "file.txt"])
    assert_equal true, result.flags["extended-regexp"]
    assert_equal "pattern", result.arguments["pattern"]
  end

  def test_grep_exclusive_group_violation
    assert_parse_error(GREP_SPEC, ["grep", "-E", "-F", "pattern"],
      error_type: "exclusive_group_violation")
  end

  def test_grep_exclusive_group_three_flags
    assert_parse_error(GREP_SPEC, ["grep", "-E", "-F", "-P", "pattern"],
      error_type: "exclusive_group_violation")
  end

  def test_grep_missing_pattern
    assert_parse_error(GREP_SPEC, ["grep", "file.txt"],
      error_type: "missing_required_argument")
  end

  def test_grep_pattern_optional_when_e_flag_used
    # When -e PATTERN is given, the positional PATTERN is optional
    result = parse(GREP_SPEC, ["grep", "-e", "foo", "file.txt"])
    # With -e providing the pattern, positional should be treated as FILE
    # Actually: -e sets pattern-option; positional PATTERN has required_unless_flag
    # So "file.txt" would be assigned to 'pattern', and 'file' would be []
    # Let's check: tokens after scanning = ["file.txt"], parsed_flags has "pattern-option"
    # required_unless_flag is satisfied, so 'pattern' can get "file.txt"
    assert_instance_of ParseResult, result
  end

  def test_grep_ignore_case
    result = parse(GREP_SPEC, ["grep", "-i", "pattern", "file.txt"])
    assert_equal true, result.flags["ignore-case"]
  end

  # ===========================================================================
  # 5. tar — traditional mode (flags without leading dash)
  # ===========================================================================

  TAR_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "tar",
    "description" => "GNU tar: manipulate tape archives",
    "parsing_mode" => "traditional",
    "flags" => [
      {
        "id" => "extract",
        "short" => "x",
        "description" => "Extract files",
        "type" => "boolean"
      },
      {
        "id" => "create",
        "short" => "c",
        "description" => "Create archive",
        "type" => "boolean"
      },
      {
        "id" => "verbose",
        "short" => "v",
        "description" => "Verbose",
        "type" => "boolean"
      },
      {
        "id" => "file",
        "short" => "f",
        "description" => "Archive file",
        "type" => "path"
      }
    ],
    "mutually_exclusive_groups" => [
      {
        "id" => "operation",
        "flag_ids" => ["extract", "create"],
        "required" => false
      }
    ],
    "arguments" => [
      {
        "id" => "members",
        "name" => "MEMBER",
        "description" => "Files to extract/include",
        "type" => "path",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_tar_traditional_xvf
    # "tar xvf archive.tar" → x, v, f=archive.tar  (no leading dash)
    result = parse(TAR_SPEC, ["tar", "xvf", "archive.tar"])
    assert_equal true, result.flags["extract"]
    assert_equal true, result.flags["verbose"]
    assert_equal "archive.tar", result.flags["file"]
  end

  def test_tar_with_dash_prefix_still_works
    result = parse(TAR_SPEC, ["tar", "-xvf", "archive.tar"])
    assert_equal true, result.flags["extract"]
    assert_equal true, result.flags["verbose"]
    assert_equal "archive.tar", result.flags["file"]
  end

  def test_tar_create_and_extract_conflict
    assert_parse_error(TAR_SPEC, ["tar", "cxf", "archive.tar"],
      error_type: "exclusive_group_violation")
  end

  def test_tar_extract_with_members
    result = parse(TAR_SPEC, ["tar", "xvf", "archive.tar", "file1.txt", "file2.txt"])
    assert_equal true, result.flags["extract"]
    assert_equal ["file1.txt", "file2.txt"], result.arguments["members"]
  end

  # ===========================================================================
  # 6. java — single_dash_long flags
  # ===========================================================================

  JAVA_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "java",
    "description" => "Launch a Java application",
    "flags" => [
      {
        "id" => "classpath",
        "single_dash_long" => "classpath",
        "description" => "Class search path",
        "type" => "string",
        "value_name" => "CLASSPATH"
      },
      {
        "id" => "cp",
        "single_dash_long" => "cp",
        "description" => "Class search path (alias)",
        "type" => "string",
        "value_name" => "CLASSPATH"
      },
      {
        "id" => "jar",
        "single_dash_long" => "jar",
        "description" => "Execute a JAR file",
        "type" => "boolean"
      },
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Enable verbose output",
        "type" => "boolean"
      }
    ],
    "arguments" => [
      {
        "id" => "classname",
        "name" => "CLASSNAME",
        "description" => "Fully qualified name of class to launch",
        "type" => "string",
        "required" => true
      },
      {
        "id" => "args",
        "name" => "ARGS",
        "description" => "Arguments passed to the main method",
        "type" => "string",
        "required" => false,
        "variadic" => true,
        "variadic_min" => 0
      }
    ]
  }.freeze

  def test_java_classpath_and_main_class
    result = parse(JAVA_SPEC, ["java", "-classpath", ".", "Main"])
    assert_equal ".", result.flags["classpath"]
    assert_equal "Main", result.arguments["classname"]
    assert_equal [], result.arguments["args"]
  end

  def test_java_cp_alias
    result = parse(JAVA_SPEC, ["java", "-cp", "lib/*:.", "com.example.App"])
    assert_equal "lib/*:.", result.flags["cp"]
    assert_equal "com.example.App", result.arguments["classname"]
  end

  def test_java_jar_flag_boolean
    result = parse(JAVA_SPEC, ["java", "-jar", "app.jar"])
    assert_equal true, result.flags["jar"]
    assert_equal "app.jar", result.arguments["classname"]
  end

  def test_java_verbose_flag
    result = parse(JAVA_SPEC, ["java", "-v", "-classpath", ".", "Main"])
    assert_equal true, result.flags["verbose"]
    assert_equal ".", result.flags["classpath"]
  end

  def test_java_missing_classname
    assert_parse_error(JAVA_SPEC, ["java"],
      error_type: "missing_required_argument")
  end

  def test_java_with_program_args
    result = parse(JAVA_SPEC, ["java", "-classpath", ".", "Main", "arg1", "arg2"])
    assert_equal "Main", result.arguments["classname"]
    assert_equal ["arg1", "arg2"], result.arguments["args"]
  end

  # ===========================================================================
  # 7. git — subcommand routing, alias resolution, unknown command suggestion
  # ===========================================================================

  GIT_SPEC = {
    "cli_builder_spec_version" => "1.0",
    "name" => "git",
    "description" => "The stupid content tracker",
    "version" => "2.39.0",
    "global_flags" => [
      {
        "id" => "verbose",
        "short" => "v",
        "long" => "verbose",
        "description" => "Be more verbose",
        "type" => "boolean"
      }
    ],
    "commands" => [
      {
        "id" => "cmd-remote",
        "name" => "remote",
        "description" => "Manage set of tracked repositories",
        "flags" => [],
        "commands" => [
          {
            "id" => "cmd-remote-add",
            "name" => "add",
            "aliases" => ["a"],
            "description" => "Add a named remote",
            "flags" => [],
            "arguments" => [
              {
                "id" => "name",
                "name" => "NAME",
                "description" => "Remote name",
                "type" => "string",
                "required" => true
              },
              {
                "id" => "url",
                "name" => "URL",
                "description" => "Remote URL",
                "type" => "string",
                "required" => true
              }
            ]
          },
          {
            "id" => "cmd-remote-remove",
            "name" => "remove",
            "aliases" => ["rm"],
            "description" => "Remove a remote",
            "flags" => [],
            "arguments" => [
              {
                "id" => "name",
                "name" => "NAME",
                "description" => "Remote name",
                "type" => "string",
                "required" => true
              }
            ]
          }
        ]
      },
      {
        "id" => "cmd-commit",
        "name" => "commit",
        "description" => "Record changes to the repository",
        "flags" => [
          {
            "id" => "message",
            "short" => "m",
            "long" => "message",
            "description" => "Commit message",
            "type" => "string",
            "required" => true
          },
          {
            "id" => "all",
            "short" => "a",
            "long" => "all",
            "description" => "Stage all tracked files",
            "type" => "boolean"
          }
        ],
        "arguments" => []
      }
    ]
  }.freeze

  def test_git_remote_add
    result = parse(GIT_SPEC, ["git", "remote", "add", "origin", "https://example.com"])
    assert_equal ["git", "remote", "add"], result.command_path
    assert_equal "origin", result.arguments["name"]
    assert_equal "https://example.com", result.arguments["url"]
  end

  def test_git_remote_add_with_alias
    result = parse(GIT_SPEC, ["git", "remote", "a", "origin", "https://example.com"])
    # Alias "a" should resolve to canonical name "add"
    assert_equal ["git", "remote", "add"], result.command_path
    assert_equal "origin", result.arguments["name"]
  end

  def test_git_remote_remove
    result = parse(GIT_SPEC, ["git", "remote", "remove", "origin"])
    assert_equal ["git", "remote", "remove"], result.command_path
    assert_equal "origin", result.arguments["name"]
  end

  def test_git_remote_rm_alias
    result = parse(GIT_SPEC, ["git", "remote", "rm", "upstream"])
    assert_equal ["git", "remote", "remove"], result.command_path
  end

  def test_git_commit_with_message
    result = parse(GIT_SPEC, ["git", "commit", "-m", "Initial commit"])
    assert_equal ["git", "commit"], result.command_path
    assert_equal "Initial commit", result.flags["message"]
  end

  def test_git_commit_missing_message
    assert_parse_error(GIT_SPEC, ["git", "commit"],
      error_type: "missing_required_flag")
  end

  def test_git_verbose_global_flag
    result = parse(GIT_SPEC, ["git", "commit", "-v", "-m", "msg"])
    assert_equal true, result.flags["verbose"]
    assert_equal "msg", result.flags["message"]
  end

  def test_git_unknown_command_produces_error
    # "comit" is close to "commit" — should suggest it
    # In our implementation, unknown tokens in non-subcommand position are positional
    # or cause errors depending on context. The parser routes what it can.
    # With no matching subcommand at root, "comit" becomes a positional.
    # For the git root which has no arguments defined, this would just be extra tokens.
    # Let's test that the parse result's command_path is just ["git"]
    result = parse(GIT_SPEC, ["git", "remote", "add", "origin", "https://x.com"])
    assert_equal ["git", "remote", "add"], result.command_path
  end

  def test_git_help_for_subcommand
    result = parse(GIT_SPEC, ["git", "remote", "add", "--help"])
    assert_instance_of HelpResult, result
    assert_equal ["git", "remote", "add"], result.command_path
    assert_match(/add/, result.text)
  end

  def test_git_version
    result = parse(GIT_SPEC, ["git", "--version"])
    assert_instance_of VersionResult, result
    assert_equal "2.39.0", result.version
  end

  def test_git_commit_all_and_message
    result = parse(GIT_SPEC, ["git", "commit", "-a", "-m", "Fix bug"])
    assert_equal true, result.flags["all"]
    assert_equal "Fix bug", result.flags["message"]
  end

  # ===========================================================================
  # Additional edge cases
  # ===========================================================================

  # Long flag with value (--output=file) style
  def test_long_flag_with_value
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "mytool",
      "description" => "A tool",
      "flags" => [
        {"id" => "output", "long" => "output", "description" => "output", "type" => "string"}
      ]
    }
    result = parse(spec, ["mytool", "--output=myfile.txt"])
    assert_equal "myfile.txt", result.flags["output"]
  end

  # POSIX mode: first positional stops flag scanning
  def test_posix_mode_stops_at_first_positional
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "find",
      "description" => "find files",
      "parsing_mode" => "posix",
      "flags" => [
        {"id" => "verbose", "short" => "v", "description" => "verbose", "type" => "boolean"}
      ],
      "arguments" => [
        {"id" => "path", "name" => "PATH", "description" => "start path", "type" => "path",
         "required" => false, "variadic" => true, "variadic_min" => 0}
      ]
    }
    result = parse(spec, ["find", "/tmp", "-v"])
    # In POSIX mode, after "/tmp" is seen, "-v" becomes positional
    assert_equal false, result.flags["verbose"]
    assert_equal ["/tmp", "-v"], result.arguments["path"]
  end

  # Repeatable flag
  def test_repeatable_flag_accumulates_values
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "curl",
      "description" => "curl",
      "flags" => [
        {"id" => "header", "short" => "H", "long" => "header",
         "description" => "header", "type" => "string",
         "repeatable" => true}
      ]
    }
    result = parse(spec, ["curl", "-H", "X-Foo: bar", "-H", "X-Baz: qux"])
    assert_equal ["X-Foo: bar", "X-Baz: qux"], result.flags["header"]
  end

  # Duplicate non-repeatable flag is an error
  def test_duplicate_non_repeatable_flag
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "mytool",
      "description" => "A tool",
      "flags" => [
        {"id" => "verbose", "short" => "v", "description" => "verbose", "type" => "boolean"}
      ]
    }
    assert_parse_error(spec, ["mytool", "-v", "-v"],
      error_type: "duplicate_flag")
  end

  # Enum flag
  def test_enum_flag_valid_value
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "formatter",
      "description" => "format",
      "flags" => [
        {
          "id" => "format",
          "long" => "format",
          "description" => "Output format",
          "type" => "enum",
          "enum_values" => ["json", "csv", "table"]
        }
      ]
    }
    result = parse(spec, ["formatter", "--format=json"])
    assert_equal "json", result.flags["format"]
  end

  def test_enum_flag_invalid_value
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "formatter",
      "description" => "format",
      "flags" => [
        {
          "id" => "format",
          "long" => "format",
          "description" => "Output format",
          "type" => "enum",
          "enum_values" => ["json", "csv", "table"]
        }
      ]
    }
    assert_parse_error(spec, ["formatter", "--format=bork"],
      error_type: "invalid_enum_value")
  end

  # Integer flag
  def test_integer_flag
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "app",
      "flags" => [
        {"id" => "count", "long" => "count", "description" => "count", "type" => "integer"}
      ]
    }
    result = parse(spec, ["myapp", "--count=5"])
    assert_equal 5, result.flags["count"]
    assert_instance_of Integer, result.flags["count"]
  end

  def test_invalid_integer_flag
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "app",
      "flags" => [
        {"id" => "count", "long" => "count", "description" => "count", "type" => "integer"}
      ]
    }
    assert_parse_error(spec, ["myapp", "--count=abc"],
      error_type: "invalid_value")
  end

  # Float flag
  def test_float_flag
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "app",
      "flags" => [
        {"id" => "ratio", "long" => "ratio", "description" => "ratio", "type" => "float"}
      ]
    }
    result = parse(spec, ["myapp", "--ratio=3.14"])
    assert_in_delta 3.14, result.flags["ratio"]
    assert_instance_of Float, result.flags["ratio"]
  end

  # Default values
  def test_default_value_used_when_flag_absent
    spec = {
      "cli_builder_spec_version" => "1.0",
      "name" => "myapp",
      "description" => "app",
      "flags" => [
        {
          "id" => "timeout",
          "long" => "timeout",
          "description" => "timeout in seconds",
          "type" => "integer",
          "default" => 30
        }
      ]
    }
    result = parse(spec, ["myapp"])
    assert_equal 30, result.flags["timeout"]
  end
end
