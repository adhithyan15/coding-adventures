# Unix Tools Roadmap

A comprehensive catalog of classic Unix/POSIX CLI tools that can be implemented
using CLI Builder, ordered from simplest to most complex. Each tool includes its
JSON spec, implementation complexity notes, and the CLI Builder features it
exercises.

---

## Design Philosophy

The Unix philosophy — "do one thing well" — aligns perfectly with CLI Builder's
separation of interface and implementation. Each tool's interface is a JSON file;
the implementation is pure business logic. One JSON spec works across all six
languages in this codebase (Go, Python, Ruby, Rust, TypeScript, Elixir).

### What Makes a Good CLI Builder Tool?

A tool is a good fit when:
- Its interface is **flags + positional arguments** (not an embedded sub-language)
- Its behavior is **deterministic** given the same inputs
- Its parsing is **stateless** (no REPL, no interactive mode)

### What's Out of Scope?

Tools with embedded sub-languages where the "argument" is actually a program:
- `sed` — transformation scripts (`s/foo/bar/g`)
- `awk` — full programming language in an argument
- `find` — predicate expression trees (`-name X -type f -newer Y`)
- Interactive tools — `vim`, `less`, `top` (terminal UI, not CLI parsing)

These could use CLI Builder for their *flag* parsing, but the sub-language
parsing is a separate problem entirely.

---

## Tier 0: Already Implemented

### pwd — Print Working Directory

**Status:** ✅ Implemented in all 6 languages
**CLI Builder features exercised:** boolean flags, mutually exclusive groups, POSIX mode

---

## Tier 1: Trivial Tools (No Arguments, Minimal Flags)

These tools have zero or near-zero flags, no positional arguments, and
trivial business logic. Perfect for onboarding new languages or verifying
the CLI Builder pipeline end-to-end.

### 1.1 true — Exit Successfully

**Complexity:** ⭐ (1/10)
**Logic:** Exit with code 0. That's it.
**CLI Builder features:** builtin_flags only (help, version)

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "true",
  "display_name": "true",
  "description": "Do nothing, successfully. Exit with status code 0.",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [],
  "commands": []
}
```

### 1.2 false — Exit Unsuccessfully

**Complexity:** ⭐ (1/10)
**Logic:** Exit with code 1. That's it.
**CLI Builder features:** builtin_flags only

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "false",
  "display_name": "false",
  "description": "Do nothing, unsuccessfully. Exit with status code 1.",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [],
  "commands": []
}
```

### 1.3 yes — Repeatedly Output a String

**Complexity:** ⭐ (1/10)
**Logic:** Print "y" (or a given string) repeatedly until killed.
**CLI Builder features:** optional positional argument

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "yes",
  "display_name": "yes",
  "description": "Repeatedly output a line with 'y' or a specified string",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [
    {
      "id": "string",
      "name": "STRING",
      "description": "The string to output repeatedly (default: 'y')",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "y"
    }
  ],
  "commands": []
}
```

### 1.4 whoami — Print Effective User Name

**Complexity:** ⭐ (1/10)
**Logic:** Print the username associated with the current effective user ID.
**CLI Builder features:** builtin_flags only

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "whoami",
  "display_name": "whoami",
  "description": "Print the user name associated with the current effective user ID",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [],
  "commands": []
}
```

### 1.5 logname — Print Login Name

**Complexity:** ⭐ (1/10)
**Logic:** Print the name of the user logged in on the controlling terminal.
**CLI Builder features:** builtin_flags only

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "logname",
  "display_name": "logname",
  "description": "Print the user's login name",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [],
  "commands": []
}
```

### 1.6 tty — Print Terminal Name

**Complexity:** ⭐ (1/10)
**Logic:** Print the file name of the terminal connected to standard input.
**CLI Builder features:** boolean flag

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tty",
  "display_name": "tty",
  "description": "Print the file name of the terminal connected to standard input",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "silent",
      "short": "s",
      "long": "silent",
      "description": "Print nothing, only return an exit status",
      "type": "boolean"
    }
  ],
  "arguments": [],
  "commands": []
}
```

### 1.7 nproc — Print Number of Processing Units

**Complexity:** ⭐ (1/10)
**Logic:** Print the number of available processing units.
**CLI Builder features:** boolean flag, integer flag

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "nproc",
  "display_name": "nproc",
  "description": "Print the number of processing units available",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "long": "all",
      "description": "Print the number of installed processors (not just available)",
      "type": "boolean"
    },
    {
      "id": "ignore",
      "long": "ignore",
      "description": "If possible, exclude N processing units",
      "type": "integer",
      "value_name": "N"
    }
  ],
  "arguments": [],
  "commands": []
}
```

### 1.8 sleep — Delay for a Specified Time

**Complexity:** ⭐ (1/10)
**Logic:** Pause execution for the specified number of seconds.
**CLI Builder features:** required positional argument, float type

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "sleep",
  "display_name": "sleep",
  "description": "Delay for a specified amount of time",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [
    {
      "id": "duration",
      "name": "NUMBER[SUFFIX]",
      "description": "Duration to sleep. SUFFIX may be 's' (seconds, default), 'm' (minutes), 'h' (hours), or 'd' (days). Multiple values are summed.",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

---

## Tier 2: Simple Tools (Few Flags, Simple I/O)

These tools introduce reading files, writing output, and a handful of
well-understood flags. The business logic is straightforward.

### 2.1 echo — Display a Line of Text

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print arguments separated by spaces, followed by a newline.
**CLI Builder features:** boolean flags, variadic string arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "echo",
  "display_name": "echo",
  "description": "Display a line of text",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "no_newline",
      "short": "n",
      "description": "Do not output the trailing newline",
      "type": "boolean"
    },
    {
      "id": "enable_escapes",
      "short": "e",
      "description": "Enable interpretation of backslash escapes",
      "type": "boolean"
    },
    {
      "id": "disable_escapes",
      "short": "E",
      "description": "Disable interpretation of backslash escapes (default)",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "strings",
      "name": "STRING",
      "description": "Strings to display",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "escape-mode",
      "flag_ids": ["enable_escapes", "disable_escapes"],
      "required": false
    }
  ]
}
```

### 2.2 cat — Concatenate and Print Files

**Complexity:** ⭐⭐ (2/10)
**Logic:** Read files sequentially and write to stdout. `-` means stdin.
**CLI Builder features:** boolean flags, variadic file arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cat",
  "display_name": "cat",
  "description": "Concatenate files and print on the standard output",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "number",
      "short": "n",
      "long": "number",
      "description": "Number all output lines",
      "type": "boolean"
    },
    {
      "id": "number_nonblank",
      "short": "b",
      "long": "number-nonblank",
      "description": "Number nonempty output lines, overrides -n",
      "type": "boolean"
    },
    {
      "id": "squeeze_blank",
      "short": "s",
      "long": "squeeze-blank",
      "description": "Suppress repeated empty output lines",
      "type": "boolean"
    },
    {
      "id": "show_tabs",
      "short": "T",
      "long": "show-tabs",
      "description": "Display TAB characters as ^I",
      "type": "boolean"
    },
    {
      "id": "show_ends",
      "short": "E",
      "long": "show-ends",
      "description": "Display $ at end of each line",
      "type": "boolean"
    },
    {
      "id": "show_nonprinting",
      "short": "v",
      "long": "show-nonprinting",
      "description": "Use ^ and M- notation, except for LFD and TAB",
      "type": "boolean"
    },
    {
      "id": "show_all",
      "short": "A",
      "long": "show-all",
      "description": "Equivalent to -vET",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to concatenate. Use '-' for standard input.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 2.3 head — Output the First Part of Files

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print the first N lines (default 10) of each file.
**CLI Builder features:** integer flag, variadic path arguments, mutually exclusive

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "head",
  "display_name": "head",
  "description": "Output the first part of files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "lines",
      "short": "n",
      "long": "lines",
      "description": "Print the first NUM lines instead of the first 10",
      "type": "integer",
      "value_name": "NUM",
      "default": 10
    },
    {
      "id": "bytes",
      "short": "c",
      "long": "bytes",
      "description": "Print the first NUM bytes of each file",
      "type": "integer",
      "value_name": "NUM"
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Never print headers giving file names",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Always print headers giving file names",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to read. Use '-' for standard input.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "count-mode",
      "flag_ids": ["lines", "bytes"],
      "required": false
    },
    {
      "id": "header-mode",
      "flag_ids": ["quiet", "verbose"],
      "required": false
    }
  ]
}
```

### 2.4 tail — Output the Last Part of Files

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print the last N lines (default 10) of each file.
**CLI Builder features:** integer flag, boolean flags, mutually exclusive groups

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tail",
  "display_name": "tail",
  "description": "Output the last part of files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "lines",
      "short": "n",
      "long": "lines",
      "description": "Output the last NUM lines instead of the last 10. Prefix NUM with '+' to output starting with line NUM.",
      "type": "string",
      "value_name": "NUM",
      "default": "10"
    },
    {
      "id": "bytes",
      "short": "c",
      "long": "bytes",
      "description": "Output the last NUM bytes",
      "type": "string",
      "value_name": "NUM"
    },
    {
      "id": "follow",
      "short": "f",
      "long": "follow",
      "description": "Output appended data as the file grows",
      "type": "boolean"
    },
    {
      "id": "retry",
      "long": "retry",
      "description": "Keep trying to open a file if it is inaccessible",
      "type": "boolean"
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Never output headers giving file names",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Always output headers giving file names",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to read. Use '-' for standard input.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "count-mode",
      "flag_ids": ["lines", "bytes"],
      "required": false
    },
    {
      "id": "header-mode",
      "flag_ids": ["quiet", "verbose"],
      "required": false
    }
  ]
}
```

### 2.5 wc — Word, Line, and Byte Count

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print line, word, and byte counts for each file.
**CLI Builder features:** multiple boolean flags, variadic file arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "wc",
  "display_name": "wc",
  "description": "Print newline, word, and byte counts for each file",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "lines",
      "short": "l",
      "long": "lines",
      "description": "Print the newline counts",
      "type": "boolean"
    },
    {
      "id": "words",
      "short": "w",
      "long": "words",
      "description": "Print the word counts",
      "type": "boolean"
    },
    {
      "id": "bytes",
      "short": "c",
      "long": "bytes",
      "description": "Print the byte counts",
      "type": "boolean"
    },
    {
      "id": "chars",
      "short": "m",
      "long": "chars",
      "description": "Print the character counts",
      "type": "boolean"
    },
    {
      "id": "max_line_length",
      "short": "L",
      "long": "max-line-length",
      "description": "Print the maximum display width",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to count. Use '-' for standard input.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "byte-char-mode",
      "flag_ids": ["bytes", "chars"],
      "required": false
    }
  ]
}
```

### 2.6 basename — Strip Directory and Suffix from Filenames

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print the last component of a pathname, optionally removing a suffix.
**CLI Builder features:** string flag, positional arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "basename",
  "display_name": "basename",
  "description": "Strip directory and suffix from filenames",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "multiple",
      "short": "a",
      "long": "multiple",
      "description": "Support multiple arguments and treat each as a NAME",
      "type": "boolean"
    },
    {
      "id": "suffix",
      "short": "s",
      "long": "suffix",
      "description": "Remove a trailing SUFFIX; implies -a",
      "type": "string",
      "value_name": "SUFFIX"
    },
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "name",
      "name": "NAME",
      "description": "The pathname(s) to process",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 2.7 dirname — Strip Last Component from Filename

**Complexity:** ⭐⭐ (2/10)
**Logic:** Output each NAME with its last non-slash component removed.
**CLI Builder features:** variadic string arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "dirname",
  "display_name": "dirname",
  "description": "Strip last component from file name",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "names",
      "name": "NAME",
      "description": "The pathname(s) to process",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 2.8 seq — Print a Sequence of Numbers

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print numbers from FIRST to LAST with INCREMENT.
**CLI Builder features:** string flags, variadic positional (1-3 numbers)

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "seq",
  "display_name": "seq",
  "description": "Print a sequence of numbers",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "separator",
      "short": "s",
      "long": "separator",
      "description": "Use STRING to separate numbers (default: newline)",
      "type": "string",
      "value_name": "STRING",
      "default": "\n"
    },
    {
      "id": "equal_width",
      "short": "w",
      "long": "equal-width",
      "description": "Equalize width by padding with leading zeroes",
      "type": "boolean"
    },
    {
      "id": "format",
      "short": "f",
      "long": "format",
      "description": "Use printf style floating-point FORMAT",
      "type": "string",
      "value_name": "FORMAT"
    }
  ],
  "arguments": [
    {
      "id": "numbers",
      "name": "LAST or FIRST LAST or FIRST INCREMENT LAST",
      "description": "1 arg: count from 1 to LAST. 2 args: count from FIRST to LAST. 3 args: count from FIRST to LAST by INCREMENT.",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1,
      "variadic_max": 3
    }
  ],
  "commands": []
}
```

### 2.9 tee — Read from Stdin, Write to Stdout and Files

**Complexity:** ⭐⭐ (2/10)
**Logic:** Copy stdin to stdout and to each specified file.
**CLI Builder features:** boolean flags, variadic path arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tee",
  "display_name": "tee",
  "description": "Read from standard input and write to standard output and files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "append",
      "short": "a",
      "long": "append",
      "description": "Append to the given files, do not overwrite",
      "type": "boolean"
    },
    {
      "id": "ignore_interrupts",
      "short": "i",
      "long": "ignore-interrupts",
      "description": "Ignore the SIGINT signal",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to write to in addition to stdout",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

### 2.10 rev — Reverse Lines of a File

**Complexity:** ⭐⭐ (2/10)
**Logic:** Reverse each line character by character.
**CLI Builder features:** variadic file arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "rev",
  "display_name": "rev",
  "description": "Reverse lines characterwise",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to reverse. Reads from stdin if none given.",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

### 2.11 printenv — Print Environment Variables

**Complexity:** ⭐⭐ (2/10)
**Logic:** Print the values of specified environment variables, or all of them.
**CLI Builder features:** boolean flag, variadic optional args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "printenv",
  "display_name": "printenv",
  "description": "Print all or part of environment",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "null",
      "short": "0",
      "long": "null",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "variables",
      "name": "VARIABLE",
      "description": "Variables to print. If none given, print all.",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

---

## Tier 3: Medium Tools (Multiple Flags, Richer Logic)

These tools have more flags, some flag interactions, and moderately
complex business logic (filesystem operations, text transformation).

### 3.1 mkdir — Make Directories

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Create directories. With `-p`, create parent directories as needed.
**CLI Builder features:** boolean flags, string flag (mode), variadic directory args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "mkdir",
  "display_name": "mkdir",
  "description": "Make directories",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "parents",
      "short": "p",
      "long": "parents",
      "description": "No error if existing, make parent directories as needed",
      "type": "boolean"
    },
    {
      "id": "mode",
      "short": "m",
      "long": "mode",
      "description": "Set file mode (as in chmod), not a=rwx - umask",
      "type": "string",
      "value_name": "MODE"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Print a message for each created directory",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "directories",
      "name": "DIRECTORY",
      "description": "Directories to create",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 3.2 rmdir — Remove Empty Directories

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Remove empty directories.
**CLI Builder features:** boolean flags, variadic directory args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "rmdir",
  "display_name": "rmdir",
  "description": "Remove empty directories",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "parents",
      "short": "p",
      "long": "parents",
      "description": "Remove DIRECTORY and its ancestors (e.g., 'rmdir -p a/b/c' removes a/b/c, a/b, and a)",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Output a diagnostic for every directory processed",
      "type": "boolean"
    },
    {
      "id": "ignore_fail",
      "long": "ignore-fail-on-non-empty",
      "description": "Ignore each failure that is solely because a directory is non-empty",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "directories",
      "name": "DIRECTORY",
      "description": "Directories to remove",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 3.3 touch — Change File Timestamps

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Update access and modification times; create files if they don't exist.
**CLI Builder features:** boolean flags, string flags, path flag (reference), conflicts

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "touch",
  "display_name": "touch",
  "description": "Change file timestamps",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "access_only",
      "short": "a",
      "description": "Change only the access time",
      "type": "boolean"
    },
    {
      "id": "no_create",
      "short": "c",
      "long": "no-create",
      "description": "Do not create any files",
      "type": "boolean"
    },
    {
      "id": "date",
      "short": "d",
      "long": "date",
      "description": "Parse STRING and use it instead of current time",
      "type": "string",
      "value_name": "STRING"
    },
    {
      "id": "modification_only",
      "short": "m",
      "description": "Change only the modification time",
      "type": "boolean"
    },
    {
      "id": "reference",
      "short": "r",
      "long": "reference",
      "description": "Use this file's times instead of current time",
      "type": "path",
      "value_name": "FILE"
    },
    {
      "id": "timestamp",
      "short": "t",
      "description": "Use [[CC]YY]MMDDhhmm[.ss] instead of current time",
      "type": "string",
      "value_name": "STAMP"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to touch",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "time-source",
      "flag_ids": ["date", "reference", "timestamp"],
      "required": false
    }
  ]
}
```

### 3.4 ln — Make Links Between Files

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Create hard or symbolic links.
**CLI Builder features:** boolean flags, path arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "ln",
  "display_name": "ln",
  "description": "Make links between files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "symbolic",
      "short": "s",
      "long": "symbolic",
      "description": "Make symbolic links instead of hard links",
      "type": "boolean"
    },
    {
      "id": "force",
      "short": "f",
      "long": "force",
      "description": "Remove existing destination files",
      "type": "boolean"
    },
    {
      "id": "interactive",
      "short": "i",
      "long": "interactive",
      "description": "Prompt whether to remove destinations",
      "type": "boolean"
    },
    {
      "id": "no_dereference",
      "short": "n",
      "long": "no-dereference",
      "description": "Treat LINK_NAME as a normal file if it is a symbolic link to a directory",
      "type": "boolean"
    },
    {
      "id": "relative",
      "short": "r",
      "long": "relative",
      "description": "Create symbolic links relative to link location",
      "type": "boolean",
      "requires": ["symbolic"]
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Print name of each linked file",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "targets",
      "name": "TARGET",
      "description": "Target file(s) or directory to link to",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 3.5 rm — Remove Files or Directories

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Remove files and directories. Dangerous — be careful.
**CLI Builder features:** boolean flags, conflicts, variadic path args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "rm",
  "display_name": "rm",
  "description": "Remove files or directories",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "force",
      "short": "f",
      "long": "force",
      "description": "Ignore nonexistent files and arguments, never prompt",
      "type": "boolean"
    },
    {
      "id": "interactive",
      "short": "i",
      "description": "Prompt before every removal",
      "type": "boolean"
    },
    {
      "id": "interactive_once",
      "short": "I",
      "description": "Prompt once before removing more than three files or when removing recursively",
      "type": "boolean"
    },
    {
      "id": "recursive",
      "short": "r",
      "long": "recursive",
      "description": "Remove directories and their contents recursively",
      "type": "boolean"
    },
    {
      "id": "dir",
      "short": "d",
      "long": "dir",
      "description": "Remove empty directories",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Explain what is being done",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to remove",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "prompt-mode",
      "flag_ids": ["force", "interactive", "interactive_once"],
      "required": false
    }
  ]
}
```

### 3.6 realpath — Print Resolved Absolute Path

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Resolve symlinks, `.` and `..` references to produce canonical path.
**CLI Builder features:** boolean flags, variadic path args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "realpath",
  "display_name": "realpath",
  "description": "Print the resolved absolute file name",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "canonicalize_existing",
      "short": "e",
      "long": "canonicalize-existing",
      "description": "All components of the path must exist",
      "type": "boolean"
    },
    {
      "id": "canonicalize_missing",
      "short": "m",
      "long": "canonicalize-missing",
      "description": "No path components need exist or be a directory",
      "type": "boolean"
    },
    {
      "id": "no_symlinks",
      "short": "s",
      "long": "no-symlinks",
      "description": "Don't expand symlinks",
      "type": "boolean"
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Suppress most error messages",
      "type": "boolean"
    },
    {
      "id": "relative_to",
      "long": "relative-to",
      "description": "Print the resolved path relative to DIR",
      "type": "string",
      "value_name": "DIR"
    },
    {
      "id": "relative_base",
      "long": "relative-base",
      "description": "Print absolute paths unless paths below DIR",
      "type": "string",
      "value_name": "DIR"
    },
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to resolve",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "canonicalize-mode",
      "flag_ids": ["canonicalize_existing", "canonicalize_missing"],
      "required": false
    }
  ]
}
```

### 3.7 tr — Translate or Delete Characters

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Replace, squeeze, or delete characters from stdin.
**CLI Builder features:** boolean flags, positional arguments (character sets)

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tr",
  "display_name": "tr",
  "description": "Translate or delete characters",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "complement",
      "short": "c",
      "long": "complement",
      "description": "Use the complement of SET1",
      "type": "boolean"
    },
    {
      "id": "delete",
      "short": "d",
      "long": "delete",
      "description": "Delete characters in SET1, do not translate",
      "type": "boolean"
    },
    {
      "id": "squeeze_repeats",
      "short": "s",
      "long": "squeeze-repeats",
      "description": "Replace each sequence of a repeated character that is listed in the last specified SET with a single occurrence",
      "type": "boolean"
    },
    {
      "id": "truncate_set1",
      "short": "t",
      "long": "truncate-set1",
      "description": "First truncate SET1 to length of SET2",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "set1",
      "name": "SET1",
      "description": "Set of characters to translate from (or delete)",
      "type": "string",
      "required": true
    },
    {
      "id": "set2",
      "name": "SET2",
      "description": "Set of characters to translate to",
      "type": "string",
      "required": false
    }
  ],
  "commands": []
}
```

### 3.8 uniq — Report or Omit Repeated Lines

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Filter adjacent matching lines, with counting and comparison options.
**CLI Builder features:** boolean flags, integer flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "uniq",
  "display_name": "uniq",
  "description": "Report or omit repeated lines",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "count",
      "short": "c",
      "long": "count",
      "description": "Prefix lines by the number of occurrences",
      "type": "boolean"
    },
    {
      "id": "repeated",
      "short": "d",
      "long": "repeated",
      "description": "Only print duplicate lines, one for each group",
      "type": "boolean"
    },
    {
      "id": "all_repeated",
      "short": "D",
      "long": "all-repeated",
      "description": "Print all duplicate lines. Delimiting done with blank lines.",
      "type": "enum",
      "enum_values": ["none", "prepend", "separate"],
      "default": "none"
    },
    {
      "id": "unique",
      "short": "u",
      "long": "unique",
      "description": "Only print unique lines",
      "type": "boolean"
    },
    {
      "id": "skip_fields",
      "short": "f",
      "long": "skip-fields",
      "description": "Avoid comparing the first N fields",
      "type": "integer",
      "value_name": "N"
    },
    {
      "id": "skip_chars",
      "short": "s",
      "long": "skip-chars",
      "description": "Avoid comparing the first N characters",
      "type": "integer",
      "value_name": "N"
    },
    {
      "id": "check_chars",
      "short": "w",
      "long": "check-chars",
      "description": "Compare no more than N characters in lines",
      "type": "integer",
      "value_name": "N"
    },
    {
      "id": "ignore_case",
      "short": "i",
      "long": "ignore-case",
      "description": "Ignore differences in case when comparing",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "input",
      "name": "INPUT",
      "description": "Input file (default: stdin)",
      "type": "string",
      "required": false
    },
    {
      "id": "output",
      "name": "OUTPUT",
      "description": "Output file (default: stdout)",
      "type": "string",
      "required": false
    }
  ],
  "commands": []
}
```

### 3.9 expand — Convert Tabs to Spaces

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Convert tab characters to spaces.
**CLI Builder features:** string flag (tab stops), variadic file args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "expand",
  "display_name": "expand",
  "description": "Convert tabs to spaces",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "initial",
      "short": "i",
      "long": "initial",
      "description": "Do not convert tabs after non blanks",
      "type": "boolean"
    },
    {
      "id": "tabs",
      "short": "t",
      "long": "tabs",
      "description": "Have tabs N characters apart, not 8. Comma-separated list for variable tab stops.",
      "type": "string",
      "value_name": "N"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to expand. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 3.10 unexpand — Convert Spaces to Tabs

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Convert spaces to tabs (inverse of expand).
**CLI Builder features:** same as expand

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "unexpand",
  "display_name": "unexpand",
  "description": "Convert spaces to tabs",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Convert all blanks, instead of just initial blanks",
      "type": "boolean"
    },
    {
      "id": "first_only",
      "long": "first-only",
      "description": "Convert only leading sequences of blanks (overrides -a)",
      "type": "boolean"
    },
    {
      "id": "tabs",
      "short": "t",
      "long": "tabs",
      "description": "Have tabs N characters apart, not 8",
      "type": "string",
      "value_name": "N"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to process. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 3.11 fold — Wrap Lines to Fit in Specified Width

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Wrap input lines so they are no wider than a specified width.
**CLI Builder features:** integer flag, boolean flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "fold",
  "display_name": "fold",
  "description": "Wrap each input line to fit in specified width",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "bytes",
      "short": "b",
      "long": "bytes",
      "description": "Count bytes rather than columns",
      "type": "boolean"
    },
    {
      "id": "spaces",
      "short": "s",
      "long": "spaces",
      "description": "Break at spaces",
      "type": "boolean"
    },
    {
      "id": "width",
      "short": "w",
      "long": "width",
      "description": "Use WIDTH columns instead of 80",
      "type": "integer",
      "value_name": "WIDTH",
      "default": 80
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to fold. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 3.12 nl — Number Lines of Files

**Complexity:** ⭐⭐⭐ (3/10)
**Logic:** Write lines with line numbers prepended.
**CLI Builder features:** string flags, integer flags, enum flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "nl",
  "display_name": "nl",
  "description": "Number lines of files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "body_numbering",
      "short": "b",
      "long": "body-numbering",
      "description": "Use STYLE for numbering body lines",
      "type": "enum",
      "enum_values": ["a", "t", "n", "pBRE"],
      "value_name": "STYLE",
      "default": "t"
    },
    {
      "id": "header_numbering",
      "short": "h",
      "long": "header-numbering",
      "description": "Use STYLE for numbering header lines",
      "type": "enum",
      "enum_values": ["a", "t", "n", "pBRE"],
      "value_name": "STYLE",
      "default": "n"
    },
    {
      "id": "footer_numbering",
      "short": "f",
      "long": "footer-numbering",
      "description": "Use STYLE for numbering footer lines",
      "type": "enum",
      "enum_values": ["a", "t", "n", "pBRE"],
      "value_name": "STYLE",
      "default": "n"
    },
    {
      "id": "line_increment",
      "short": "i",
      "long": "line-increment",
      "description": "Line number increment at each line",
      "type": "integer",
      "value_name": "NUMBER",
      "default": 1
    },
    {
      "id": "number_format",
      "short": "n",
      "long": "number-format",
      "description": "Insert line numbers according to FORMAT",
      "type": "enum",
      "enum_values": ["ln", "rn", "rz"],
      "value_name": "FORMAT",
      "default": "rn"
    },
    {
      "id": "number_width",
      "short": "w",
      "long": "number-width",
      "description": "Use NUMBER columns for line numbers",
      "type": "integer",
      "value_name": "NUMBER",
      "default": 6
    },
    {
      "id": "number_separator",
      "short": "s",
      "long": "number-separator",
      "description": "Add STRING after (possible) line number",
      "type": "string",
      "value_name": "STRING",
      "default": "\t"
    },
    {
      "id": "starting_line_number",
      "short": "v",
      "long": "starting-line-number",
      "description": "First line number on each logical page",
      "type": "integer",
      "value_name": "NUMBER",
      "default": 1
    },
    {
      "id": "section_delimiter",
      "short": "d",
      "long": "section-delimiter",
      "description": "Use CC for logical page delimiters",
      "type": "string",
      "value_name": "CC",
      "default": "\\:"
    },
    {
      "id": "no_renumber",
      "short": "p",
      "long": "no-renumber",
      "description": "Do not reset line numbers for each section",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to number. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

---

## Tier 4: Moderate Tools (Complex Flag Interactions, Richer Output)

These tools have more complex logic: sorting algorithms, permission
parsing, columnar output, or filesystem traversal.

### 4.1 sort — Sort Lines of Text Files

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Sort lines according to various keys, orderings, and modes.
**CLI Builder features:** many flags, enum-like keys, conflicts, variadic files

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "sort",
  "display_name": "sort",
  "description": "Sort lines of text files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "reverse",
      "short": "r",
      "long": "reverse",
      "description": "Reverse the result of comparisons",
      "type": "boolean"
    },
    {
      "id": "numeric_sort",
      "short": "n",
      "long": "numeric-sort",
      "description": "Compare according to string numerical value",
      "type": "boolean"
    },
    {
      "id": "human_numeric_sort",
      "short": "h",
      "long": "human-numeric-sort",
      "description": "Compare human readable numbers (e.g., 2K 1G)",
      "type": "boolean"
    },
    {
      "id": "month_sort",
      "short": "M",
      "long": "month-sort",
      "description": "Compare (unknown) < 'JAN' < ... < 'DEC'",
      "type": "boolean"
    },
    {
      "id": "general_numeric_sort",
      "short": "g",
      "long": "general-numeric-sort",
      "description": "Compare according to general numerical value",
      "type": "boolean"
    },
    {
      "id": "version_sort",
      "short": "V",
      "long": "version-sort",
      "description": "Natural sort of (version) numbers within text",
      "type": "boolean"
    },
    {
      "id": "unique",
      "short": "u",
      "long": "unique",
      "description": "With -c, check for strict ordering; without -c, output only the first of an equal run",
      "type": "boolean"
    },
    {
      "id": "ignore_case",
      "short": "f",
      "long": "ignore-case",
      "description": "Fold lower case to upper case characters",
      "type": "boolean"
    },
    {
      "id": "dictionary_order",
      "short": "d",
      "long": "dictionary-order",
      "description": "Consider only blanks and alphanumeric characters",
      "type": "boolean"
    },
    {
      "id": "ignore_nonprinting",
      "short": "i",
      "long": "ignore-nonprinting",
      "description": "Consider only printable characters",
      "type": "boolean"
    },
    {
      "id": "ignore_leading_blanks",
      "short": "b",
      "long": "ignore-leading-blanks",
      "description": "Ignore leading blanks",
      "type": "boolean"
    },
    {
      "id": "stable",
      "short": "s",
      "long": "stable",
      "description": "Stabilize sort by disabling last-resort comparison",
      "type": "boolean"
    },
    {
      "id": "key",
      "short": "k",
      "long": "key",
      "description": "Sort via a key; KEYDEF gives location and type",
      "type": "string",
      "value_name": "KEYDEF",
      "repeatable": true
    },
    {
      "id": "field_separator",
      "short": "t",
      "long": "field-separator",
      "description": "Use SEP instead of non-blank to blank transition",
      "type": "string",
      "value_name": "SEP"
    },
    {
      "id": "output",
      "short": "o",
      "long": "output",
      "description": "Write result to FILE instead of stdout",
      "type": "string",
      "value_name": "FILE"
    },
    {
      "id": "check",
      "short": "c",
      "long": "check",
      "description": "Check for sorted input; do not sort",
      "type": "boolean"
    },
    {
      "id": "merge",
      "short": "m",
      "long": "merge",
      "description": "Merge already sorted files; do not sort",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to sort. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 4.2 cut — Remove Sections from Lines

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Select fields, bytes, or characters from each line.
**CLI Builder features:** mutually exclusive groups (required), string flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cut",
  "display_name": "cut",
  "description": "Remove sections from each line of files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "bytes",
      "short": "b",
      "long": "bytes",
      "description": "Select only these bytes",
      "type": "string",
      "value_name": "LIST"
    },
    {
      "id": "characters",
      "short": "c",
      "long": "characters",
      "description": "Select only these characters",
      "type": "string",
      "value_name": "LIST"
    },
    {
      "id": "fields",
      "short": "f",
      "long": "fields",
      "description": "Select only these fields",
      "type": "string",
      "value_name": "LIST"
    },
    {
      "id": "delimiter",
      "short": "d",
      "long": "delimiter",
      "description": "Use DELIM instead of TAB for field delimiter",
      "type": "string",
      "value_name": "DELIM"
    },
    {
      "id": "only_delimited",
      "short": "s",
      "long": "only-delimited",
      "description": "Do not print lines not containing delimiters",
      "type": "boolean"
    },
    {
      "id": "output_delimiter",
      "long": "output-delimiter",
      "description": "Use STRING as the output delimiter",
      "type": "string",
      "value_name": "STRING"
    },
    {
      "id": "complement",
      "long": "complement",
      "description": "Complement the set of selected bytes, characters, or fields",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to cut. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "selection-mode",
      "flag_ids": ["bytes", "characters", "fields"],
      "required": true
    }
  ]
}
```

### 4.3 paste — Merge Lines of Files

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Merge corresponding lines from multiple files, separated by tabs.
**CLI Builder features:** string flag, boolean flag, variadic files

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "paste",
  "display_name": "paste",
  "description": "Merge lines of files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "delimiters",
      "short": "d",
      "long": "delimiters",
      "description": "Reuse characters from LIST instead of TABs",
      "type": "string",
      "value_name": "LIST"
    },
    {
      "id": "serial",
      "short": "s",
      "long": "serial",
      "description": "Paste one file at a time instead of in parallel",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to merge. Use '-' for stdin.",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 4.4 comm — Compare Two Sorted Files Line by Line

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Output three columns: lines only in file1, only in file2, and in both.
**CLI Builder features:** boolean flags used as column suppressors

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "comm",
  "display_name": "comm",
  "description": "Compare two sorted files line by line",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "suppress_col1",
      "short": "1",
      "description": "Suppress column 1 (lines unique to FILE1)",
      "type": "boolean"
    },
    {
      "id": "suppress_col2",
      "short": "2",
      "description": "Suppress column 2 (lines unique to FILE2)",
      "type": "boolean"
    },
    {
      "id": "suppress_col3",
      "short": "3",
      "description": "Suppress column 3 (lines that appear in both files)",
      "type": "boolean"
    },
    {
      "id": "check_order",
      "long": "check-order",
      "description": "Check that the input is correctly sorted, even if all input lines are pairable",
      "type": "boolean"
    },
    {
      "id": "nocheck_order",
      "long": "nocheck-order",
      "description": "Do not check that the input is correctly sorted",
      "type": "boolean"
    },
    {
      "id": "output_delimiter",
      "long": "output-delimiter",
      "description": "Separate columns with STRING",
      "type": "string",
      "value_name": "STRING"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file1",
      "name": "FILE1",
      "description": "First sorted file",
      "type": "string",
      "required": true
    },
    {
      "id": "file2",
      "name": "FILE2",
      "description": "Second sorted file",
      "type": "string",
      "required": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "order-check",
      "flag_ids": ["check_order", "nocheck_order"],
      "required": false
    }
  ]
}
```

### 4.5 uname — Print System Information

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Print system info (kernel name, hostname, version, architecture, etc.).
**CLI Builder features:** many boolean flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "uname",
  "display_name": "uname",
  "description": "Print system information",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Print all information, in the following order",
      "type": "boolean"
    },
    {
      "id": "kernel_name",
      "short": "s",
      "long": "kernel-name",
      "description": "Print the kernel name",
      "type": "boolean"
    },
    {
      "id": "nodename",
      "short": "n",
      "long": "nodename",
      "description": "Print the network node hostname",
      "type": "boolean"
    },
    {
      "id": "kernel_release",
      "short": "r",
      "long": "kernel-release",
      "description": "Print the kernel release",
      "type": "boolean"
    },
    {
      "id": "kernel_version",
      "short": "v",
      "long": "kernel-version",
      "description": "Print the kernel version",
      "type": "boolean"
    },
    {
      "id": "machine",
      "short": "m",
      "long": "machine",
      "description": "Print the machine hardware name",
      "type": "boolean"
    },
    {
      "id": "processor",
      "short": "p",
      "long": "processor",
      "description": "Print the processor type (non-portable)",
      "type": "boolean"
    },
    {
      "id": "hardware_platform",
      "short": "i",
      "long": "hardware-platform",
      "description": "Print the hardware platform (non-portable)",
      "type": "boolean"
    },
    {
      "id": "operating_system",
      "short": "o",
      "long": "operating-system",
      "description": "Print the operating system",
      "type": "boolean"
    }
  ],
  "arguments": [],
  "commands": []
}
```

### 4.6 id — Print User and Group Information

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Print real and effective user and group IDs.
**CLI Builder features:** boolean flags, optional positional arg

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "id",
  "display_name": "id",
  "description": "Print real and effective user and group IDs",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "user",
      "short": "u",
      "long": "user",
      "description": "Print only the effective user ID",
      "type": "boolean"
    },
    {
      "id": "group",
      "short": "g",
      "long": "group",
      "description": "Print only the effective group ID",
      "type": "boolean"
    },
    {
      "id": "groups",
      "short": "G",
      "long": "groups",
      "description": "Print all group IDs",
      "type": "boolean"
    },
    {
      "id": "name",
      "short": "n",
      "long": "name",
      "description": "Print a name instead of a number, for -ugG",
      "type": "boolean",
      "required_unless": ["user", "group", "groups"]
    },
    {
      "id": "real",
      "short": "r",
      "long": "real",
      "description": "Print the real ID instead of the effective ID, with -ugG",
      "type": "boolean"
    },
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "Delimit entries with NUL characters, not whitespace",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "user_name",
      "name": "USER",
      "description": "User to look up (default: current user)",
      "type": "string",
      "required": false
    }
  ],
  "commands": []
}
```

### 4.7 groups — Print Group Names

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Print the groups a user belongs to.
**CLI Builder features:** variadic optional args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "groups",
  "display_name": "groups",
  "description": "Print the groups a user is in",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [],
  "arguments": [
    {
      "id": "users",
      "name": "USERNAME",
      "description": "Users to look up (default: current user)",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

### 4.8 df — Report Filesystem Disk Space Usage

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Display disk usage statistics for each filesystem.
**CLI Builder features:** many boolean flags, mutually exclusive groups

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "df",
  "display_name": "df",
  "description": "Report file system disk space usage",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Include pseudo, duplicate, inaccessible file systems",
      "type": "boolean"
    },
    {
      "id": "human_readable",
      "short": "h",
      "long": "human-readable",
      "description": "Print sizes in powers of 1024 (e.g., 1023M)",
      "type": "boolean"
    },
    {
      "id": "si",
      "short": "H",
      "long": "si",
      "description": "Print sizes in powers of 1000 (e.g., 1.1G)",
      "type": "boolean"
    },
    {
      "id": "inodes",
      "short": "i",
      "long": "inodes",
      "description": "List inode information instead of block usage",
      "type": "boolean"
    },
    {
      "id": "block_size",
      "short": "B",
      "long": "block-size",
      "description": "Scale sizes by SIZE before printing them",
      "type": "string",
      "value_name": "SIZE"
    },
    {
      "id": "local",
      "short": "l",
      "long": "local",
      "description": "Limit listing to local file systems",
      "type": "boolean"
    },
    {
      "id": "portability",
      "short": "P",
      "long": "portability",
      "description": "Use the POSIX output format",
      "type": "boolean"
    },
    {
      "id": "type",
      "short": "t",
      "long": "type",
      "description": "Limit listing to file systems of TYPE",
      "type": "string",
      "value_name": "TYPE"
    },
    {
      "id": "exclude_type",
      "short": "x",
      "long": "exclude-type",
      "description": "Limit listing to file systems not of TYPE",
      "type": "string",
      "value_name": "TYPE"
    },
    {
      "id": "print_type",
      "short": "T",
      "long": "print-type",
      "description": "Print file system type",
      "type": "boolean"
    },
    {
      "id": "total",
      "long": "total",
      "description": "Elide all entries insignificant to available space, and produce a grand total",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Show information about the file system on which each FILE resides",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "size-format",
      "flag_ids": ["human_readable", "si"],
      "required": false
    }
  ]
}
```

### 4.9 du — Estimate File Space Usage

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Summarize disk usage of the set of files, recursing into directories.
**CLI Builder features:** integer flags, boolean flags, string flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "du",
  "display_name": "du",
  "description": "Estimate file space usage",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Write counts for all files, not just directories",
      "type": "boolean"
    },
    {
      "id": "human_readable",
      "short": "h",
      "long": "human-readable",
      "description": "Print sizes in human readable format (e.g., 1K 234M 2G)",
      "type": "boolean"
    },
    {
      "id": "si",
      "long": "si",
      "description": "Like -h but use powers of 1000 not 1024",
      "type": "boolean"
    },
    {
      "id": "summarize",
      "short": "s",
      "long": "summarize",
      "description": "Display only a total for each argument",
      "type": "boolean"
    },
    {
      "id": "total",
      "short": "c",
      "long": "total",
      "description": "Produce a grand total",
      "type": "boolean"
    },
    {
      "id": "max_depth",
      "short": "d",
      "long": "max-depth",
      "description": "Print the total for a directory only if it is N or fewer levels below the command line argument",
      "type": "integer",
      "value_name": "N"
    },
    {
      "id": "block_size",
      "short": "B",
      "long": "block-size",
      "description": "Scale sizes by SIZE before printing",
      "type": "string",
      "value_name": "SIZE"
    },
    {
      "id": "dereference",
      "short": "L",
      "long": "dereference",
      "description": "Dereference all symbolic links",
      "type": "boolean"
    },
    {
      "id": "no_dereference",
      "short": "P",
      "long": "no-dereference",
      "description": "Don't follow any symbolic links (default)",
      "type": "boolean"
    },
    {
      "id": "one_file_system",
      "short": "x",
      "long": "one-file-system",
      "description": "Skip directories on different file systems",
      "type": "boolean"
    },
    {
      "id": "exclude",
      "long": "exclude",
      "description": "Exclude files that match PATTERN",
      "type": "string",
      "value_name": "PATTERN",
      "repeatable": true
    },
    {
      "id": "null",
      "short": "0",
      "long": "null",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to measure",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "."
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "size-format",
      "flag_ids": ["human_readable", "si"],
      "required": false
    },
    {
      "id": "deref-mode",
      "flag_ids": ["dereference", "no_dereference"],
      "required": false
    }
  ]
}
```

### 4.10 md5sum — Compute and Check MD5 Message Digest

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Compute or verify MD5 hashes.
**CLI Builder features:** boolean flags, variadic file args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "md5sum",
  "display_name": "md5sum",
  "description": "Compute and check MD5 message digest",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "check",
      "short": "c",
      "long": "check",
      "description": "Read MD5 sums from the files and check them",
      "type": "boolean"
    },
    {
      "id": "binary",
      "short": "b",
      "long": "binary",
      "description": "Read in binary mode",
      "type": "boolean"
    },
    {
      "id": "text",
      "short": "t",
      "long": "text",
      "description": "Read in text mode (default)",
      "type": "boolean"
    },
    {
      "id": "quiet",
      "long": "quiet",
      "description": "Don't print OK for each successfully verified file",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "status",
      "long": "status",
      "description": "Don't output anything, status code shows success",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "strict",
      "long": "strict",
      "description": "Exit non-zero for improperly formatted checksum lines",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "warn",
      "short": "w",
      "long": "warn",
      "description": "Warn about improperly formatted checksum lines",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to checksum. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "read-mode",
      "flag_ids": ["binary", "text"],
      "required": false
    }
  ]
}
```

### 4.11 sha256sum — Compute and Check SHA-256 Message Digest

**Complexity:** ⭐⭐⭐⭐ (4/10)
**Logic:** Identical structure to md5sum but using SHA-256.
**CLI Builder features:** same as md5sum — demonstrates spec reuse

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "sha256sum",
  "display_name": "sha256sum",
  "description": "Compute and check SHA256 message digest",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "check",
      "short": "c",
      "long": "check",
      "description": "Read SHA256 sums from the files and check them",
      "type": "boolean"
    },
    {
      "id": "binary",
      "short": "b",
      "long": "binary",
      "description": "Read in binary mode",
      "type": "boolean"
    },
    {
      "id": "text",
      "short": "t",
      "long": "text",
      "description": "Read in text mode (default)",
      "type": "boolean"
    },
    {
      "id": "quiet",
      "long": "quiet",
      "description": "Don't print OK for each successfully verified file",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "status",
      "long": "status",
      "description": "Don't output anything, status code shows success",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "strict",
      "long": "strict",
      "description": "Exit non-zero for improperly formatted checksum lines",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "warn",
      "short": "w",
      "long": "warn",
      "description": "Warn about improperly formatted checksum lines",
      "type": "boolean",
      "requires": ["check"]
    },
    {
      "id": "zero",
      "short": "z",
      "long": "zero",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to checksum. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "-"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "read-mode",
      "flag_ids": ["binary", "text"],
      "required": false
    }
  ]
}
```

---

## Tier 5: Complex Tools (Rich Flag Interactions, Filesystem Traversal)

These tools have deep flag interactions, complex output formatting,
or require significant algorithmic logic.

### 5.1 cp — Copy Files and Directories

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** Copy files/directories with preservation of attributes, recursive traversal.
**CLI Builder features:** many flags, conflicts, requires, variadic source + trailing dest

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cp",
  "display_name": "cp",
  "description": "Copy files and directories",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "archive",
      "short": "a",
      "long": "archive",
      "description": "Same as -dR --preserve=all",
      "type": "boolean"
    },
    {
      "id": "force",
      "short": "f",
      "long": "force",
      "description": "If an existing destination file cannot be opened, remove it and try again",
      "type": "boolean"
    },
    {
      "id": "interactive",
      "short": "i",
      "long": "interactive",
      "description": "Prompt before overwrite",
      "type": "boolean"
    },
    {
      "id": "no_clobber",
      "short": "n",
      "long": "no-clobber",
      "description": "Do not overwrite an existing file",
      "type": "boolean"
    },
    {
      "id": "recursive",
      "short": "R",
      "long": "recursive",
      "description": "Copy directories recursively",
      "type": "boolean"
    },
    {
      "id": "no_dereference",
      "short": "d",
      "description": "Same as --no-dereference --preserve=links",
      "type": "boolean"
    },
    {
      "id": "dereference",
      "short": "L",
      "long": "dereference",
      "description": "Always follow symbolic links in SOURCE",
      "type": "boolean"
    },
    {
      "id": "preserve",
      "long": "preserve",
      "description": "Preserve the specified attributes (mode, ownership, timestamps, context, links, xattr, all)",
      "type": "string",
      "value_name": "ATTR_LIST"
    },
    {
      "id": "no_preserve",
      "long": "no-preserve",
      "description": "Don't preserve the specified attributes",
      "type": "string",
      "value_name": "ATTR_LIST"
    },
    {
      "id": "link",
      "short": "l",
      "long": "link",
      "description": "Hard link files instead of copying",
      "type": "boolean"
    },
    {
      "id": "symbolic_link",
      "short": "s",
      "long": "symbolic-link",
      "description": "Make symbolic links instead of copying",
      "type": "boolean"
    },
    {
      "id": "update",
      "short": "u",
      "long": "update",
      "description": "Copy only when the SOURCE file is newer than the destination file or when the destination file is missing",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Explain what is being done",
      "type": "boolean"
    },
    {
      "id": "target_directory",
      "short": "t",
      "long": "target-directory",
      "description": "Copy all SOURCE arguments into DIRECTORY",
      "type": "string",
      "value_name": "DIRECTORY"
    },
    {
      "id": "no_target_directory",
      "short": "T",
      "long": "no-target-directory",
      "description": "Treat DEST as a normal file",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "sources",
      "name": "SOURCE",
      "description": "Source file(s) and destination. Last argument is the destination.",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 2
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "overwrite-mode",
      "flag_ids": ["force", "interactive", "no_clobber"],
      "required": false
    },
    {
      "id": "target-mode",
      "flag_ids": ["target_directory", "no_target_directory"],
      "required": false
    },
    {
      "id": "copy-mode",
      "flag_ids": ["link", "symbolic_link"],
      "required": false
    }
  ]
}
```

### 5.2 mv — Move (Rename) Files

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** Move or rename files and directories.
**CLI Builder features:** similar to cp — conflicts, variadic source + trailing dest

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "mv",
  "display_name": "mv",
  "description": "Move (rename) files",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "force",
      "short": "f",
      "long": "force",
      "description": "Do not prompt before overwriting",
      "type": "boolean"
    },
    {
      "id": "interactive",
      "short": "i",
      "long": "interactive",
      "description": "Prompt before overwrite",
      "type": "boolean"
    },
    {
      "id": "no_clobber",
      "short": "n",
      "long": "no-clobber",
      "description": "Do not overwrite an existing file",
      "type": "boolean"
    },
    {
      "id": "update",
      "short": "u",
      "long": "update",
      "description": "Move only when the SOURCE file is newer than the destination file or when the destination file is missing",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Explain what is being done",
      "type": "boolean"
    },
    {
      "id": "target_directory",
      "short": "t",
      "long": "target-directory",
      "description": "Move all SOURCE arguments into DIRECTORY",
      "type": "string",
      "value_name": "DIRECTORY"
    },
    {
      "id": "no_target_directory",
      "short": "T",
      "long": "no-target-directory",
      "description": "Treat DEST as a normal file",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "sources",
      "name": "SOURCE",
      "description": "Source file(s) and destination. Last argument is the destination.",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 2
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "overwrite-mode",
      "flag_ids": ["force", "interactive", "no_clobber"],
      "required": false
    },
    {
      "id": "target-mode",
      "flag_ids": ["target_directory", "no_target_directory"],
      "required": false
    }
  ]
}
```

### 5.3 ls — List Directory Contents

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** List directory contents with rich formatting options.
**CLI Builder features:** many flags (20+), many mutually exclusive groups, enum flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "ls",
  "display_name": "ls",
  "description": "List directory contents",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Do not ignore entries starting with .",
      "type": "boolean"
    },
    {
      "id": "almost_all",
      "short": "A",
      "long": "almost-all",
      "description": "Do not list implied . and ..",
      "type": "boolean"
    },
    {
      "id": "long",
      "short": "l",
      "description": "Use a long listing format",
      "type": "boolean"
    },
    {
      "id": "human_readable",
      "short": "h",
      "long": "human-readable",
      "description": "With -l, print sizes like 1K 234M 2G",
      "type": "boolean"
    },
    {
      "id": "si",
      "long": "si",
      "description": "Like -h but use powers of 1000 not 1024",
      "type": "boolean"
    },
    {
      "id": "reverse",
      "short": "r",
      "long": "reverse",
      "description": "Reverse order while sorting",
      "type": "boolean"
    },
    {
      "id": "recursive",
      "short": "R",
      "long": "recursive",
      "description": "List subdirectories recursively",
      "type": "boolean"
    },
    {
      "id": "sort_by_size",
      "short": "S",
      "description": "Sort by file size, largest first",
      "type": "boolean"
    },
    {
      "id": "sort_by_time",
      "short": "t",
      "description": "Sort by time, newest first",
      "type": "boolean"
    },
    {
      "id": "sort_by_extension",
      "short": "X",
      "description": "Sort alphabetically by entry extension",
      "type": "boolean"
    },
    {
      "id": "sort_by_version",
      "short": "v",
      "description": "Natural sort of (version) numbers within text",
      "type": "boolean"
    },
    {
      "id": "unsorted",
      "short": "U",
      "description": "Do not sort; list entries in directory order",
      "type": "boolean"
    },
    {
      "id": "directory",
      "short": "d",
      "long": "directory",
      "description": "List directories themselves, not their contents",
      "type": "boolean"
    },
    {
      "id": "classify",
      "short": "F",
      "long": "classify",
      "description": "Append indicator (one of */=>@|) to entries",
      "type": "boolean"
    },
    {
      "id": "inode",
      "short": "i",
      "long": "inode",
      "description": "Print the index number of each file",
      "type": "boolean"
    },
    {
      "id": "no_group",
      "short": "G",
      "long": "no-group",
      "description": "In a long listing, don't print group names",
      "type": "boolean"
    },
    {
      "id": "numeric_uid_gid",
      "short": "n",
      "long": "numeric-uid-gid",
      "description": "Like -l, but list numeric user and group IDs",
      "type": "boolean"
    },
    {
      "id": "one_per_line",
      "short": "1",
      "description": "List one file per line",
      "type": "boolean"
    },
    {
      "id": "color",
      "long": "color",
      "description": "Colorize the output",
      "type": "enum",
      "enum_values": ["always", "auto", "never"],
      "default": "auto"
    },
    {
      "id": "block_size",
      "long": "block-size",
      "description": "With -l, scale sizes by SIZE",
      "type": "string",
      "value_name": "SIZE"
    },
    {
      "id": "dereference",
      "short": "L",
      "long": "dereference",
      "description": "Show information for the file the symlink references",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to list",
      "type": "string",
      "required": false,
      "variadic": true,
      "default": "."
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "dot-files",
      "flag_ids": ["all", "almost_all"],
      "required": false
    },
    {
      "id": "size-format",
      "flag_ids": ["human_readable", "si"],
      "required": false
    },
    {
      "id": "sort-mode",
      "flag_ids": ["sort_by_size", "sort_by_time", "sort_by_extension", "sort_by_version", "unsorted"],
      "required": false
    }
  ]
}
```

### 5.4 grep — Print Lines That Match Patterns

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** Search for patterns in files. Patterns are data (not embedded sub-language).
**CLI Builder features:** many flags, mutually exclusive groups, repeatable flags, requires

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "grep",
  "display_name": "grep",
  "description": "Print lines that match patterns",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "extended_regexp",
      "short": "E",
      "long": "extended-regexp",
      "description": "Interpret PATTERNS as extended regular expressions",
      "type": "boolean"
    },
    {
      "id": "fixed_strings",
      "short": "F",
      "long": "fixed-strings",
      "description": "Interpret PATTERNS as fixed strings, not regular expressions",
      "type": "boolean"
    },
    {
      "id": "basic_regexp",
      "short": "G",
      "long": "basic-regexp",
      "description": "Interpret PATTERNS as basic regular expressions (default)",
      "type": "boolean"
    },
    {
      "id": "perl_regexp",
      "short": "P",
      "long": "perl-regexp",
      "description": "Interpret PATTERNS as Perl-compatible regular expressions",
      "type": "boolean"
    },
    {
      "id": "regexp",
      "short": "e",
      "long": "regexp",
      "description": "Use PATTERNS for matching",
      "type": "string",
      "value_name": "PATTERNS",
      "repeatable": true
    },
    {
      "id": "file",
      "short": "f",
      "long": "file",
      "description": "Take PATTERNS from FILE",
      "type": "string",
      "value_name": "FILE",
      "repeatable": true
    },
    {
      "id": "ignore_case",
      "short": "i",
      "long": "ignore-case",
      "description": "Ignore case distinctions in patterns and data",
      "type": "boolean"
    },
    {
      "id": "invert_match",
      "short": "v",
      "long": "invert-match",
      "description": "Select non-matching lines",
      "type": "boolean"
    },
    {
      "id": "word_regexp",
      "short": "w",
      "long": "word-regexp",
      "description": "Select only those lines containing matches that form whole words",
      "type": "boolean"
    },
    {
      "id": "line_regexp",
      "short": "x",
      "long": "line-regexp",
      "description": "Select only those matches that exactly match the whole line",
      "type": "boolean"
    },
    {
      "id": "count",
      "short": "c",
      "long": "count",
      "description": "Print a count of matching lines for each input file",
      "type": "boolean"
    },
    {
      "id": "files_with_matches",
      "short": "l",
      "long": "files-with-matches",
      "description": "Print only names of files with selected lines",
      "type": "boolean"
    },
    {
      "id": "files_without_match",
      "short": "L",
      "long": "files-without-match",
      "description": "Print only names of files with no selected lines",
      "type": "boolean"
    },
    {
      "id": "max_count",
      "short": "m",
      "long": "max-count",
      "description": "Stop reading a file after NUM matching lines",
      "type": "integer",
      "value_name": "NUM"
    },
    {
      "id": "only_matching",
      "short": "o",
      "long": "only-matching",
      "description": "Print only the matched (non-empty) parts of a matching line",
      "type": "boolean"
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Suppress all normal output",
      "type": "boolean"
    },
    {
      "id": "line_number",
      "short": "n",
      "long": "line-number",
      "description": "Prefix each line of output with the 1-based line number",
      "type": "boolean"
    },
    {
      "id": "with_filename",
      "short": "H",
      "long": "with-filename",
      "description": "Print the file name for each match",
      "type": "boolean"
    },
    {
      "id": "no_filename",
      "short": "h",
      "long": "no-filename",
      "description": "Suppress the prefixing of file names on output",
      "type": "boolean"
    },
    {
      "id": "after_context",
      "short": "A",
      "long": "after-context",
      "description": "Print NUM lines of trailing context after matching lines",
      "type": "integer",
      "value_name": "NUM"
    },
    {
      "id": "before_context",
      "short": "B",
      "long": "before-context",
      "description": "Print NUM lines of leading context before matching lines",
      "type": "integer",
      "value_name": "NUM"
    },
    {
      "id": "context",
      "short": "C",
      "long": "context",
      "description": "Print NUM lines of output context",
      "type": "integer",
      "value_name": "NUM"
    },
    {
      "id": "recursive",
      "short": "r",
      "long": "recursive",
      "description": "Read all files under each directory, recursively",
      "type": "boolean"
    },
    {
      "id": "dereference_recursive",
      "short": "R",
      "long": "dereference-recursive",
      "description": "Like -r but follow all symlinks",
      "type": "boolean"
    },
    {
      "id": "include",
      "long": "include",
      "description": "Search only files matching GLOB",
      "type": "string",
      "value_name": "GLOB",
      "repeatable": true
    },
    {
      "id": "exclude",
      "long": "exclude",
      "description": "Skip files matching GLOB",
      "type": "string",
      "value_name": "GLOB",
      "repeatable": true
    },
    {
      "id": "exclude_dir",
      "long": "exclude-dir",
      "description": "Skip directories matching GLOB",
      "type": "string",
      "value_name": "GLOB",
      "repeatable": true
    },
    {
      "id": "color",
      "long": "color",
      "description": "Use markers to highlight the matching strings",
      "type": "enum",
      "enum_values": ["always", "auto", "never"],
      "default": "auto"
    }
  ],
  "arguments": [
    {
      "id": "pattern",
      "name": "PATTERN",
      "description": "Pattern to search for (if -e is not used)",
      "type": "string",
      "required": false,
      "required_unless_flag": ["regexp", "file"]
    },
    {
      "id": "files",
      "name": "FILE",
      "description": "Files to search. Use '-' for stdin.",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "regex-mode",
      "flag_ids": ["extended_regexp", "fixed_strings", "basic_regexp", "perl_regexp"],
      "required": false
    },
    {
      "id": "output-files-mode",
      "flag_ids": ["files_with_matches", "files_without_match"],
      "required": false
    },
    {
      "id": "filename-mode",
      "flag_ids": ["with_filename", "no_filename"],
      "required": false
    },
    {
      "id": "recurse-mode",
      "flag_ids": ["recursive", "dereference_recursive"],
      "required": false
    }
  ]
}
```

### 5.5 join — Join Lines of Two Files on a Common Field

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** For each pair of input lines with identical join fields, write an output line.
**CLI Builder features:** many flags, string flags, integer flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "join",
  "display_name": "join",
  "description": "Join lines of two files on a common field",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "field1",
      "short": "1",
      "description": "Join on this FIELD of file 1",
      "type": "integer",
      "value_name": "FIELD"
    },
    {
      "id": "field2",
      "short": "2",
      "description": "Join on this FIELD of file 2",
      "type": "integer",
      "value_name": "FIELD"
    },
    {
      "id": "join_field",
      "short": "j",
      "description": "Equivalent to -1 FIELD -2 FIELD",
      "type": "integer",
      "value_name": "FIELD"
    },
    {
      "id": "unpaired",
      "short": "a",
      "description": "Also print unpairable lines from file FILENUM (1 or 2)",
      "type": "string",
      "value_name": "FILENUM",
      "repeatable": true
    },
    {
      "id": "only_unpaired",
      "short": "v",
      "description": "Like -a FILENUM, but suppress joined output lines",
      "type": "string",
      "value_name": "FILENUM"
    },
    {
      "id": "empty",
      "short": "e",
      "description": "Replace missing input fields with EMPTY",
      "type": "string",
      "value_name": "EMPTY"
    },
    {
      "id": "format",
      "short": "o",
      "description": "Obey FORMAT while constructing output line",
      "type": "string",
      "value_name": "FORMAT"
    },
    {
      "id": "separator",
      "short": "t",
      "description": "Use CHAR as input and output field separator",
      "type": "string",
      "value_name": "CHAR"
    },
    {
      "id": "ignore_case",
      "short": "i",
      "long": "ignore-case",
      "description": "Ignore differences in case when comparing fields",
      "type": "boolean"
    },
    {
      "id": "check_order",
      "long": "check-order",
      "description": "Check that the input is correctly sorted",
      "type": "boolean"
    },
    {
      "id": "nocheck_order",
      "long": "nocheck-order",
      "description": "Do not check that the input is correctly sorted",
      "type": "boolean"
    },
    {
      "id": "header",
      "long": "header",
      "description": "Treat the first line in each file as field headers",
      "type": "boolean"
    },
    {
      "id": "zero_terminated",
      "short": "z",
      "long": "zero-terminated",
      "description": "Line delimiter is NUL, not newline",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file1",
      "name": "FILE1",
      "description": "First sorted file",
      "type": "string",
      "required": true
    },
    {
      "id": "file2",
      "name": "FILE2",
      "description": "Second sorted file",
      "type": "string",
      "required": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "order-check",
      "flag_ids": ["check_order", "nocheck_order"],
      "required": false
    }
  ]
}
```

### 5.6 split — Split a File into Pieces

**Complexity:** ⭐⭐⭐⭐⭐ (5/10)
**Logic:** Split a file into pieces by line count, byte count, or chunk count.
**CLI Builder features:** mutually exclusive groups, integer/string flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "split",
  "display_name": "split",
  "description": "Split a file into pieces",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "lines",
      "short": "l",
      "long": "lines",
      "description": "Put NUMBER lines/records per output file",
      "type": "integer",
      "value_name": "NUMBER",
      "default": 1000
    },
    {
      "id": "bytes",
      "short": "b",
      "long": "bytes",
      "description": "Put SIZE bytes per output file",
      "type": "string",
      "value_name": "SIZE"
    },
    {
      "id": "number",
      "short": "n",
      "long": "number",
      "description": "Generate CHUNKS output files",
      "type": "string",
      "value_name": "CHUNKS"
    },
    {
      "id": "suffix_length",
      "short": "a",
      "long": "suffix-length",
      "description": "Generate suffixes of length N (default 2)",
      "type": "integer",
      "value_name": "N",
      "default": 2
    },
    {
      "id": "numeric_suffixes",
      "short": "d",
      "long": "numeric-suffixes",
      "description": "Use numeric suffixes starting at 0, not alphabetic",
      "type": "boolean"
    },
    {
      "id": "hex_suffixes",
      "short": "x",
      "long": "hex-suffixes",
      "description": "Use hex suffixes starting at 0, not alphabetic",
      "type": "boolean"
    },
    {
      "id": "additional_suffix",
      "long": "additional-suffix",
      "description": "Append an additional SUFFIX to file names",
      "type": "string",
      "value_name": "SUFFIX"
    },
    {
      "id": "verbose",
      "long": "verbose",
      "description": "Print a diagnostic just before each output file is opened",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file",
      "name": "FILE",
      "description": "File to split (default: stdin)",
      "type": "string",
      "required": false,
      "default": "-"
    },
    {
      "id": "prefix",
      "name": "PREFIX",
      "description": "Output file prefix (default: 'x')",
      "type": "string",
      "required": false,
      "default": "x"
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "split-mode",
      "flag_ids": ["lines", "bytes", "number"],
      "required": false
    },
    {
      "id": "suffix-type",
      "flag_ids": ["numeric_suffixes", "hex_suffixes"],
      "required": false
    }
  ]
}
```

---

## Tier 6: Advanced Tools (Complex Output, System Interaction)

These tools require deeper system interaction, complex formatting,
or sophisticated algorithmic logic.

### 6.1 diff — Compare Files Line by Line

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Compare two files and output their differences in various formats.
**CLI Builder features:** many flags, enum flags, mutually exclusive groups

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "diff",
  "display_name": "diff",
  "description": "Compare files line by line",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "unified",
      "short": "u",
      "long": "unified",
      "description": "Output NUM (default 3) lines of unified context",
      "type": "integer",
      "value_name": "NUM",
      "default": 3
    },
    {
      "id": "context_format",
      "short": "c",
      "long": "context",
      "description": "Output NUM (default 3) lines of copied context",
      "type": "integer",
      "value_name": "NUM",
      "default": 3
    },
    {
      "id": "ed",
      "short": "e",
      "long": "ed",
      "description": "Output an ed script",
      "type": "boolean"
    },
    {
      "id": "normal",
      "long": "normal",
      "description": "Output a normal diff (default)",
      "type": "boolean"
    },
    {
      "id": "side_by_side",
      "short": "y",
      "long": "side-by-side",
      "description": "Output in two columns",
      "type": "boolean"
    },
    {
      "id": "width",
      "short": "W",
      "long": "width",
      "description": "Output at most NUM print columns (default 130)",
      "type": "integer",
      "value_name": "NUM",
      "default": 130
    },
    {
      "id": "recursive",
      "short": "r",
      "long": "recursive",
      "description": "Recursively compare any subdirectories found",
      "type": "boolean"
    },
    {
      "id": "new_file",
      "short": "N",
      "long": "new-file",
      "description": "Treat absent files as empty",
      "type": "boolean"
    },
    {
      "id": "brief",
      "short": "q",
      "long": "brief",
      "description": "Report only when files differ",
      "type": "boolean"
    },
    {
      "id": "ignore_case",
      "short": "i",
      "long": "ignore-case",
      "description": "Ignore case differences in file contents",
      "type": "boolean"
    },
    {
      "id": "ignore_space_change",
      "short": "b",
      "long": "ignore-space-change",
      "description": "Ignore changes in the amount of white space",
      "type": "boolean"
    },
    {
      "id": "ignore_all_space",
      "short": "w",
      "long": "ignore-all-space",
      "description": "Ignore all white space",
      "type": "boolean"
    },
    {
      "id": "ignore_blank_lines",
      "short": "B",
      "long": "ignore-blank-lines",
      "description": "Ignore changes whose lines are all blank",
      "type": "boolean"
    },
    {
      "id": "exclude",
      "short": "x",
      "long": "exclude",
      "description": "Exclude files that match PATTERN",
      "type": "string",
      "value_name": "PATTERN",
      "repeatable": true
    },
    {
      "id": "color",
      "long": "color",
      "description": "Colorize the output",
      "type": "enum",
      "enum_values": ["always", "auto", "never"],
      "default": "auto"
    }
  ],
  "arguments": [
    {
      "id": "file1",
      "name": "FILE1",
      "description": "First file or directory",
      "type": "string",
      "required": true
    },
    {
      "id": "file2",
      "name": "FILE2",
      "description": "Second file or directory",
      "type": "string",
      "required": true
    }
  ],
  "commands": []
}
```

### 6.2 cmp — Compare Two Files Byte by Byte

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Compare two files byte by byte and report first difference.
**CLI Builder features:** boolean flags, integer flags

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cmp",
  "display_name": "cmp",
  "description": "Compare two files byte by byte",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "print_bytes",
      "short": "b",
      "long": "print-bytes",
      "description": "Print differing bytes",
      "type": "boolean"
    },
    {
      "id": "list",
      "short": "l",
      "long": "verbose",
      "description": "Output byte numbers and differing byte values",
      "type": "boolean"
    },
    {
      "id": "silent",
      "short": "s",
      "long": "silent",
      "description": "Output nothing; yield exit status only",
      "type": "boolean"
    },
    {
      "id": "ignore_initial",
      "short": "i",
      "long": "ignore-initial",
      "description": "Skip the first SKIP bytes of both files",
      "type": "string",
      "value_name": "SKIP"
    },
    {
      "id": "max_bytes",
      "short": "n",
      "long": "bytes",
      "description": "Compare at most LIMIT bytes",
      "type": "integer",
      "value_name": "LIMIT"
    }
  ],
  "arguments": [
    {
      "id": "file1",
      "name": "FILE1",
      "description": "First file to compare",
      "type": "string",
      "required": true
    },
    {
      "id": "file2",
      "name": "FILE2",
      "description": "Second file to compare (default: stdin)",
      "type": "string",
      "required": false,
      "default": "-"
    }
  ],
  "commands": []
}
```

### 6.3 xargs — Build and Execute Command Lines from Stdin

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Read items from stdin and execute a command with those items as arguments.
**CLI Builder features:** many flags, variadic command + args

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "xargs",
  "display_name": "xargs",
  "description": "Build and execute command lines from standard input",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "null",
      "short": "0",
      "long": "null",
      "description": "Items are separated by a null character, not whitespace; disables quote and backslash processing",
      "type": "boolean"
    },
    {
      "id": "arg_file",
      "short": "a",
      "long": "arg-file",
      "description": "Read items from FILE instead of standard input",
      "type": "string",
      "value_name": "FILE"
    },
    {
      "id": "delimiter",
      "short": "d",
      "long": "delimiter",
      "description": "Items in the input stream are separated by DELIM",
      "type": "string",
      "value_name": "DELIM"
    },
    {
      "id": "eof",
      "short": "E",
      "long": "eof",
      "description": "Set end of file string to EOF_STR",
      "type": "string",
      "value_name": "EOF_STR"
    },
    {
      "id": "replace",
      "short": "I",
      "long": "replace",
      "description": "Replace occurrences of REPLACE_STR in the initial arguments with names read from stdin",
      "type": "string",
      "value_name": "REPLACE_STR"
    },
    {
      "id": "max_lines",
      "short": "L",
      "long": "max-lines",
      "description": "Use at most MAX_LINES non-blank input lines per command line",
      "type": "integer",
      "value_name": "MAX_LINES"
    },
    {
      "id": "max_args",
      "short": "n",
      "long": "max-args",
      "description": "Use at most MAX_ARGS arguments per command line",
      "type": "integer",
      "value_name": "MAX_ARGS"
    },
    {
      "id": "max_procs",
      "short": "P",
      "long": "max-procs",
      "description": "Run up to MAX_PROCS processes at a time (0 = as many as possible)",
      "type": "integer",
      "value_name": "MAX_PROCS",
      "default": 1
    },
    {
      "id": "interactive",
      "short": "p",
      "long": "interactive",
      "description": "Prompt the user before running each command line",
      "type": "boolean"
    },
    {
      "id": "no_run_if_empty",
      "short": "r",
      "long": "no-run-if-empty",
      "description": "If the standard input does not contain any nonblanks, do not run the command",
      "type": "boolean"
    },
    {
      "id": "max_chars",
      "short": "s",
      "long": "max-chars",
      "description": "Use at most MAX_CHARS characters per command line",
      "type": "integer",
      "value_name": "MAX_CHARS"
    },
    {
      "id": "verbose",
      "short": "t",
      "long": "verbose",
      "description": "Print the command line on standard error before executing it",
      "type": "boolean"
    },
    {
      "id": "exit",
      "short": "x",
      "long": "exit",
      "description": "Exit if the size is exceeded",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "command",
      "name": "COMMAND",
      "description": "Command to execute (default: /bin/echo)",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

### 6.4 env — Run a Program in a Modified Environment

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Set environment variables and execute a command, or print the environment.
**CLI Builder features:** repeatable string flags, variadic command arguments

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "env",
  "display_name": "env",
  "description": "Run a program in a modified environment",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "ignore_environment",
      "short": "i",
      "long": "ignore-environment",
      "description": "Start with an empty environment",
      "type": "boolean"
    },
    {
      "id": "null",
      "short": "0",
      "long": "null",
      "description": "End each output line with NUL, not newline",
      "type": "boolean"
    },
    {
      "id": "unset",
      "short": "u",
      "long": "unset",
      "description": "Remove variable from the environment",
      "type": "string",
      "value_name": "NAME",
      "repeatable": true
    },
    {
      "id": "chdir",
      "short": "C",
      "long": "chdir",
      "description": "Change working directory to DIR",
      "type": "string",
      "value_name": "DIR"
    }
  ],
  "arguments": [
    {
      "id": "assignments_and_command",
      "name": "NAME=VALUE | COMMAND [ARG]...",
      "description": "Environment variable assignments and/or command to run. NAME=VALUE pairs set variables; the first non-assignment token starts the command.",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": []
}
```

### 6.5 chmod — Change File Mode Bits

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Change file permissions. The MODE argument is a mini-language (symbolic/octal), but simple enough to implement inline.
**CLI Builder features:** boolean flags, required positional + variadic files

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "chmod",
  "display_name": "chmod",
  "description": "Change file mode bits",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "recursive",
      "short": "R",
      "long": "recursive",
      "description": "Change files and directories recursively",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Output a diagnostic for every file processed",
      "type": "boolean"
    },
    {
      "id": "changes",
      "short": "c",
      "long": "changes",
      "description": "Like verbose but report only when a change is made",
      "type": "boolean"
    },
    {
      "id": "silent",
      "short": "f",
      "long": "silent",
      "description": "Suppress most error messages",
      "type": "boolean"
    },
    {
      "id": "reference",
      "long": "reference",
      "description": "Use RFILE's mode instead of MODE values",
      "type": "path",
      "value_name": "RFILE"
    }
  ],
  "arguments": [
    {
      "id": "mode",
      "name": "MODE",
      "description": "File mode in octal (e.g., 755) or symbolic notation (e.g., u+rwx,go+rx)",
      "type": "string",
      "required": true,
      "required_unless_flag": ["reference"]
    },
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to change",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": []
}
```

### 6.6 chown — Change File Owner and Group

**Complexity:** ⭐⭐⭐⭐⭐⭐ (6/10)
**Logic:** Change the user and/or group ownership of files.
**CLI Builder features:** boolean flags, OWNER[:GROUP] parsing in business logic

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "chown",
  "display_name": "chown",
  "description": "Change file owner and group",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "recursive",
      "short": "R",
      "long": "recursive",
      "description": "Operate on files and directories recursively",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Output a diagnostic for every file processed",
      "type": "boolean"
    },
    {
      "id": "changes",
      "short": "c",
      "long": "changes",
      "description": "Like verbose but report only when a change is made",
      "type": "boolean"
    },
    {
      "id": "silent",
      "short": "f",
      "long": "silent",
      "description": "Suppress most error messages",
      "type": "boolean"
    },
    {
      "id": "dereference",
      "long": "dereference",
      "description": "Affect the referent of each symbolic link, rather than the link itself (default)",
      "type": "boolean"
    },
    {
      "id": "no_dereference",
      "short": "h",
      "long": "no-dereference",
      "description": "Affect symbolic links instead of any referenced file",
      "type": "boolean"
    },
    {
      "id": "reference",
      "long": "reference",
      "description": "Use RFILE's owner and group rather than specifying OWNER:GROUP",
      "type": "path",
      "value_name": "RFILE"
    }
  ],
  "arguments": [
    {
      "id": "owner_group",
      "name": "OWNER[:GROUP]",
      "description": "New owner and optionally group, in the form OWNER, OWNER:GROUP, OWNER:, :GROUP, or OWNER.GROUP",
      "type": "string",
      "required": true,
      "required_unless_flag": ["reference"]
    },
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to change",
      "type": "string",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "deref-mode",
      "flag_ids": ["dereference", "no_dereference"],
      "required": false
    }
  ]
}
```

### 6.7 tar — Archive Utility (Traditional Mode)

**Complexity:** ⭐⭐⭐⭐⭐⭐⭐ (7/10)
**Logic:** Create, extract, list, and manipulate tape archives.
**CLI Builder features:** traditional parsing mode (no leading dash), many flags, mutually exclusive operation groups

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tar",
  "display_name": "tar",
  "description": "An archiving utility",
  "version": "1.0.0",
  "parsing_mode": "traditional",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "create",
      "short": "c",
      "long": "create",
      "description": "Create a new archive",
      "type": "boolean"
    },
    {
      "id": "extract",
      "short": "x",
      "long": "extract",
      "description": "Extract files from an archive",
      "type": "boolean"
    },
    {
      "id": "list",
      "short": "t",
      "long": "list",
      "description": "List the contents of an archive",
      "type": "boolean"
    },
    {
      "id": "append",
      "short": "r",
      "long": "append",
      "description": "Append files to the end of an archive",
      "type": "boolean"
    },
    {
      "id": "update",
      "short": "u",
      "long": "update",
      "description": "Only append files newer than copy in archive",
      "type": "boolean"
    },
    {
      "id": "file",
      "short": "f",
      "long": "file",
      "description": "Use ARCHIVE as the archive file",
      "type": "string",
      "value_name": "ARCHIVE"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Verbosely list files processed",
      "type": "boolean"
    },
    {
      "id": "gzip",
      "short": "z",
      "long": "gzip",
      "description": "Filter the archive through gzip",
      "type": "boolean"
    },
    {
      "id": "bzip2",
      "short": "j",
      "long": "bzip2",
      "description": "Filter the archive through bzip2",
      "type": "boolean"
    },
    {
      "id": "xz",
      "short": "J",
      "long": "xz",
      "description": "Filter the archive through xz",
      "type": "boolean"
    },
    {
      "id": "directory",
      "short": "C",
      "long": "directory",
      "description": "Change to DIR before performing any operations",
      "type": "string",
      "value_name": "DIR"
    },
    {
      "id": "exclude",
      "long": "exclude",
      "description": "Exclude files matching PATTERN",
      "type": "string",
      "value_name": "PATTERN",
      "repeatable": true
    },
    {
      "id": "keep_old_files",
      "short": "k",
      "long": "keep-old-files",
      "description": "Don't replace existing files when extracting",
      "type": "boolean"
    },
    {
      "id": "preserve_permissions",
      "short": "p",
      "long": "preserve-permissions",
      "description": "Extract information about file permissions",
      "type": "boolean"
    },
    {
      "id": "strip_components",
      "long": "strip-components",
      "description": "Strip NUMBER leading components from file names on extraction",
      "type": "integer",
      "value_name": "NUMBER"
    },
    {
      "id": "wildcards",
      "long": "wildcards",
      "description": "Use wildcards (default for exclusion)",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "files",
      "name": "FILE",
      "description": "Files or directories to archive/extract",
      "type": "string",
      "required": false,
      "variadic": true
    }
  ],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "operation",
      "flag_ids": ["create", "extract", "list", "append", "update"],
      "required": true
    },
    {
      "id": "compression",
      "flag_ids": ["gzip", "bzip2", "xz"],
      "required": false
    }
  ]
}
```

---

## Tier 7: Out of Scope (Embedded Sub-Languages)

These tools cannot be fully modeled by CLI Builder because their arguments
contain embedded programming languages or expression trees. CLI Builder could
handle their *flag parsing*, but the sub-language parsing requires a separate
system.

| Tool | Embedded Sub-Language | Notes |
|------|----------------------|-------|
| **sed** | Transformation scripts (`s/foo/bar/g`, address ranges) | The `-e` argument is a full stream-editing language |
| **awk** | Full programming language | The positional argument is an awk program |
| **find** | Predicate expression trees (`-name X -type f -newer Y`) | Predicates form a boolean expression tree with operators |
| **perl -e** | Perl one-liners | The argument is Perl code |
| **less / more** | Interactive pager commands | Terminal UI with keystroke-driven navigation |
| **vim / nano** | Interactive editors | Full modal/modeless editing environments |
| **top / htop** | Interactive process viewers | Live-updating terminal UI |

---

## Summary Statistics

| Tier | Count | Complexity | Status |
|------|-------|------------|--------|
| 0: Already Done | 1 | ⭐ | ✅ pwd |
| 1: Trivial | 8 | ⭐ | Pending |
| 2: Simple | 11 | ⭐⭐ | Pending |
| 3: Medium | 12 | ⭐⭐⭐ | Pending |
| 4: Moderate | 11 | ⭐⭐⭐⭐ | Pending |
| 5: Complex | 6 | ⭐⭐⭐⭐⭐ | Pending |
| 6: Advanced | 7 | ⭐⭐⭐⭐⭐⭐–⭐⭐⭐⭐⭐⭐⭐ | Pending |
| 7: Out of Scope | 7 | N/A | Won't do |
| **Total implementable** | **56** | | |

## Implementation Order

Each tool gets implemented in all 6 languages (Go, Python, Ruby, Rust,
TypeScript, Elixir) before moving to the next. This means each tool is
actually 6 implementations sharing one JSON spec.

**Recommended next tools** (in order):
1. `true` / `false` — verify pipeline with zero logic
2. `echo` — first tool with actual output logic
3. `cat` — introduces file reading
4. `wc` — introduces counting/formatting
5. `head` / `tail` — introduces line-based file slicing
6. `mkdir` / `rmdir` — introduces filesystem mutation
7. `ls` — the "final boss" of Tier 5, exercises nearly every CLI Builder feature

Each implementation follows the workflow: JSON spec → tests → implementation →
changelog → README → commit.
