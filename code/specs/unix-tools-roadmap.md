# Unix Tools Roadmap — CLI Builder Programs

A prioritized catalog of Unix/POSIX tools to reimplement using CLI Builder,
ordered from simplest to most complex. Each tool's interface is defined by a
JSON spec; only the business logic needs to be written per language.

---

## How This List Is Organized

Each tool is rated on two axes:

1. **CLI Complexity** — how many flags, arguments, subcommands, and constraints
   the JSON spec needs to describe.
2. **Logic Complexity** — how much business logic the program needs beyond
   parsing arguments.

Tools are grouped into tiers. Within each tier, they're ordered by total
implementation effort (CLI + logic combined).

### What CLI Builder Can Handle

Per the [CLI Builder Spec](cli-builder-spec.md) §1.3, the library targets ~85%
of CLI tools. It handles:

- Boolean, string, integer, float, path, file, directory, and enum flags
- Short (`-v`), long (`--verbose`), and single-dash-long (`-classpath`) forms
- Stacked short flags (`-xvf`)
- Flag values: `--output file`, `--output=file`, `-ofile`
- Variadic positional arguments with min/max constraints
- Mutually exclusive flag groups
- Flag dependencies (`--format` requires `--output`)
- Subcommands with arbitrary nesting
- Four parsing modes: POSIX, GNU, subcommand-first, traditional (tar-style)
- Auto-generated help and version output

### What Falls Outside CLI Builder's Scope

- **Embedded sub-languages**: sed scripts, awk programs, find predicates
- **Interactive/TUI tools**: vim, less, top, htop
- **REPL tools**: shells (bash, zsh), interpreters (python, node)

---

## Tier 1: Trivial (1–2 flags, minimal logic)

These are the simplest possible Unix tools. Each one can be fully implemented
in under 30 lines of business logic per language. Perfect for validating the
CLI Builder integration pattern.

### 1.1 `true` / `false`

**What it does:** Exit with status 0 (true) or 1 (false). That's it.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` (auto-injected) |
| Arguments | None |
| Logic | One line: `exit(0)` or `exit(1)` |
| Why build it | Absolute minimum viable CLI Builder program |

### 1.2 `yes`

**What it does:** Repeatedly output a string (default: "y") until killed.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` |
| Arguments | Optional variadic string (joined with spaces) |
| Logic | Infinite loop printing to stdout |
| Why build it | Tests variadic optional positional arguments |

### 1.3 `echo`

**What it does:** Print arguments to stdout.

| Aspect | Details |
|--------|---------|
| Flags | `-n` (no trailing newline), `-e` (interpret escapes), `-E` (no escapes, default) |
| Arguments | Variadic string |
| Logic | Join args with spaces, optionally interpret `\n`, `\t`, etc. |
| Why build it | Tests escape sequence handling; ubiquitous tool |

### 1.4 `whoami`

**What it does:** Print the current username.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` |
| Arguments | None |
| Logic | Read effective user ID, look up username |
| Why build it | Near-zero logic; system info lookup |

### 1.5 `logname`

**What it does:** Print the login name.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` |
| Arguments | None |
| Logic | Read `LOGNAME` env var or `getlogin()` |
| Why build it | Similar to whoami but different mechanism |

### 1.6 `tty`

**What it does:** Print the terminal name connected to stdin.

| Aspect | Details |
|--------|---------|
| Flags | `-s` / `--silent` (suppress output, exit status only) |
| Arguments | None |
| Logic | Call `ttyname(0)` or equivalent |
| Why build it | Tests boolean flag that modifies output behavior |

### 1.7 `nproc`

**What it does:** Print the number of available processors.

| Aspect | Details |
|--------|---------|
| Flags | `--all` (show all, not just available), `--ignore=N` (subtract N) |
| Arguments | None |
| Logic | Read CPU count from OS |
| Why build it | Tests integer flag with default |

### 1.8 `sleep`

**What it does:** Pause for a specified duration.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` |
| Arguments | Required number (seconds) |
| Logic | Parse number, sleep |
| Why build it | Tests required positional argument with numeric type |

---

## Tier 2: Simple (3–8 flags, straightforward logic)

These tools have a handful of flags and do one clear thing. The business logic
is straightforward — mostly file I/O or string manipulation.

### 2.1 `basename`

**What it does:** Strip directory and optionally suffix from a filename.

| Aspect | Details |
|--------|---------|
| Flags | `-a` (multiple args), `-s SUFFIX` (remove suffix), `-z` (NUL-delimited) |
| Arguments | 1 path (or variadic with `-a`), optional suffix |
| Logic | String manipulation: strip last `/`, strip suffix |

### 2.2 `dirname`

**What it does:** Strip the last component from a path.

| Aspect | Details |
|--------|---------|
| Flags | `-z` (NUL-delimited output) |
| Arguments | Variadic path |
| Logic | String manipulation: find last `/` |

### 2.3 `realpath`

**What it does:** Resolve absolute path, resolving symlinks.

| Aspect | Details |
|--------|---------|
| Flags | `-e` (all must exist), `-m` (no component needs exist), `-s` (no symlink resolution), `-q` (quiet), `--relative-to`, `--relative-base` |
| Arguments | Variadic path |
| Logic | Symlink resolution, path canonicalization |

### 2.4 `readlink`

**What it does:** Print the target of a symbolic link.

| Aspect | Details |
|--------|---------|
| Flags | `-f` (canonicalize), `-e` (canonical, all must exist), `-m` (canonical, no existence check), `-n` (no newline), `-q` (quiet), `-z` (NUL-delimited) |
| Arguments | Variadic path |
| Logic | Symlink reading, optional canonicalization |

### 2.5 `cat`

**What it does:** Concatenate files to stdout.

| Aspect | Details |
|--------|---------|
| Flags | `-n` (number lines), `-b` (number non-blank), `-s` (squeeze blank), `-E` (show $), `-T` (show ^I), `-A` (show all) |
| Arguments | Variadic file (or stdin if none) |
| Logic | Read files, optional line numbering/formatting |
| Why build it | Foundation for many other tools; tests stdin fallback |

### 2.6 `tee`

**What it does:** Copy stdin to stdout and to files.

| Aspect | Details |
|--------|---------|
| Flags | `-a` (append), `-i` (ignore SIGINT) |
| Arguments | Variadic file |
| Logic | Read stdin, write to stdout + each file |

### 2.7 `wc`

**What it does:** Count lines, words, characters, bytes.

| Aspect | Details |
|--------|---------|
| Flags | `-l` (lines), `-w` (words), `-c` (bytes), `-m` (chars), `-L` (max line length) |
| Arguments | Variadic file |
| Logic | Counting loops; format output in columns |

### 2.8 `head`

**What it does:** Output the first N lines or bytes of a file.

| Aspect | Details |
|--------|---------|
| Flags | `-n N` (lines, default 10), `-c N` (bytes), `-q` (no headers), `-v` (always headers) |
| Arguments | Variadic file |
| Logic | Read and truncate; multi-file headers |

### 2.9 `tail`

**What it does:** Output the last N lines or bytes of a file.

| Aspect | Details |
|--------|---------|
| Flags | `-n N`, `-c N`, `-f` (follow), `-F` (follow + retry), `-q`, `-v`, `--pid=PID` |
| Arguments | Variadic file |
| Logic | Seek to end, buffer lines; `-f` requires event loop |
| Note | `-f` (follow) adds significant complexity — can defer to a v2 |

### 2.10 `touch`

**What it does:** Create files or update timestamps.

| Aspect | Details |
|--------|---------|
| Flags | `-a` (access time only), `-m` (modification time only), `-c` (no create), `-t STAMP`, `-d DATE`, `-r REF_FILE` |
| Arguments | Variadic file |
| Logic | Create or update file timestamps |

### 2.11 `mkdir`

**What it does:** Create directories.

| Aspect | Details |
|--------|---------|
| Flags | `-p` (parents), `-m MODE` (permissions), `-v` (verbose) |
| Arguments | Variadic directory |
| Logic | Create directories, optionally recursive |

### 2.12 `rmdir`

**What it does:** Remove empty directories.

| Aspect | Details |
|--------|---------|
| Flags | `-p` (parents), `-v` (verbose), `--ignore-fail-on-non-empty` |
| Arguments | Variadic directory |
| Logic | Remove empty dirs, optionally walk up |

### 2.13 `seq`

**What it does:** Print a sequence of numbers.

| Aspect | Details |
|--------|---------|
| Flags | `-w` (equal width), `-f FORMAT` (printf format), `-s SEP` (separator) |
| Arguments | 1–3 numbers (LAST, or FIRST LAST, or FIRST INCREMENT LAST) |
| Logic | Number generation loop with formatting |

### 2.14 `printenv`

**What it does:** Print environment variables.

| Aspect | Details |
|--------|---------|
| Flags | `-0` (NUL-delimited) |
| Arguments | Optional variadic variable name |
| Logic | Read env vars, filter, print |

### 2.15 `env`

**What it does:** Run a command with modified environment.

| Aspect | Details |
|--------|---------|
| Flags | `-i` (empty env), `-u VAR` (unset), `-0` (NUL-delimited), `-S` (split string) |
| Arguments | Variadic: NAME=VALUE pairs followed by COMMAND and its args |
| Logic | Modify environment, exec child process |

---

## Tier 3: Moderate (5–15 flags, non-trivial logic)

These tools have richer flag sets and the business logic involves algorithms
(sorting, deduplication, column extraction, etc.).

### 3.1 `uniq`

**What it does:** Filter or report duplicate adjacent lines.

| Aspect | Details |
|--------|---------|
| Flags | `-c` (count), `-d` (only dupes), `-u` (only unique), `-i` (ignore case), `-f N` (skip fields), `-s N` (skip chars), `-w N` (compare width) |
| Arguments | Optional input file, optional output file |
| Logic | Adjacent line comparison with field/char skipping |

### 3.2 `tr`

**What it does:** Translate or delete characters.

| Aspect | Details |
|--------|---------|
| Flags | `-c` (complement), `-d` (delete), `-s` (squeeze repeats) |
| Arguments | SET1 (required), SET2 (optional depending on flags) |
| Logic | Character-by-character translation, range expansion (`a-z`), character class handling (`[:upper:]`) |

### 3.3 `cut`

**What it does:** Extract sections from each line.

| Aspect | Details |
|--------|---------|
| Flags | `-b LIST` (bytes), `-c LIST` (chars), `-f LIST` (fields), `-d DELIM` (delimiter), `-s` (only delimited lines), `--complement`, `--output-delimiter` |
| Exclusive groups | `-b`, `-c`, `-f` are mutually exclusive |
| Arguments | Variadic file |
| Logic | Range list parsing (1-5,8,12-), field extraction |

### 3.4 `paste`

**What it does:** Merge lines of files side by side.

| Aspect | Details |
|--------|---------|
| Flags | `-d LIST` (delimiter list), `-s` (serial — one file per line), `-z` (NUL-delimited) |
| Arguments | Variadic file |
| Logic | Round-robin line reading across files |

### 3.5 `sort`

**What it does:** Sort lines of text.

| Aspect | Details |
|--------|---------|
| Flags | `-r` (reverse), `-n` (numeric), `-f` (fold case), `-u` (unique), `-k KEYDEF` (sort key), `-t SEP` (field separator), `-o FILE` (output), `-d` (dictionary), `-h` (human numeric), `-M` (month), `-V` (version), `-s` (stable), `-c` (check sorted), `-m` (merge) |
| Arguments | Variadic file |
| Logic | Multi-key sorting with type-aware comparators |

### 3.6 `expand` / `unexpand`

**What it does:** Convert tabs to/from spaces.

| Aspect | Details |
|--------|---------|
| Flags | `-t N` or `-t LIST` (tab stops), `-i` (initial tabs only for unexpand) |
| Arguments | Variadic file |
| Logic | Tab-stop tracking, character replacement |

### 3.7 `fold`

**What it does:** Wrap lines to a specified width.

| Aspect | Details |
|--------|---------|
| Flags | `-w N` (width, default 80), `-s` (break at spaces), `-b` (count bytes, not columns) |
| Arguments | Variadic file |
| Logic | Line breaking with word-boundary awareness |

### 3.8 `nl`

**What it does:** Number lines of a file.

| Aspect | Details |
|--------|---------|
| Flags | `-b STYLE` (body numbering), `-h STYLE` (header), `-f STYLE` (footer), `-v N` (start), `-i N` (increment), `-n FORMAT` (ln/rn/rz), `-w N` (width), `-s SEP`, `-d DELIM` (section delimiter) |
| Arguments | Optional file |
| Logic | Section-aware line numbering with format control |

### 3.9 `comm`

**What it does:** Compare two sorted files line by line.

| Aspect | Details |
|--------|---------|
| Flags | `-1` (suppress col 1), `-2` (suppress col 2), `-3` (suppress col 3), `-i` (ignore case), `-z` (NUL-delimited) |
| Arguments | Exactly 2 files |
| Logic | Merge-style comparison of sorted inputs |

### 3.10 `md5sum` / `sha256sum`

**What it does:** Compute or verify cryptographic checksums.

| Aspect | Details |
|--------|---------|
| Flags | `-b` (binary), `-t` (text), `-c` (check), `--quiet`, `--status`, `--strict`, `-w` (warn) |
| Arguments | Variadic file |
| Logic | Hash computation, verification against check files |

### 3.11 `rm`

**What it does:** Remove files and directories.

| Aspect | Details |
|--------|---------|
| Flags | `-f` (force), `-i` (interactive), `-I` (prompt once), `-r`/`-R` (recursive), `-v` (verbose), `-d` (empty dirs), `--preserve-root`, `--one-file-system` |
| Arguments | Variadic path |
| Logic | Recursive deletion, interactive prompting, safety checks |

### 3.12 `cp`

**What it does:** Copy files and directories.

| Aspect | Details |
|--------|---------|
| Flags | `-r`/`-R` (recursive), `-a` (archive = -dR --preserve=all), `-p` (preserve), `-f` (force), `-i` (interactive), `-v` (verbose), `-l` (hard link), `-s` (symlink), `-u` (update), `-n` (no clobber), `--sparse`, `-T` (no target dir) |
| Arguments | Variadic source + 1 destination |
| Logic | Recursive copy, permission preservation, sparse file handling |

### 3.13 `mv`

**What it does:** Move or rename files.

| Aspect | Details |
|--------|---------|
| Flags | `-f` (force), `-i` (interactive), `-n` (no clobber), `-v` (verbose), `-u` (update), `-T` (no target dir) |
| Arguments | Variadic source + 1 destination |
| Logic | Rename or cross-device copy+delete |

### 3.14 `ln`

**What it does:** Create links.

| Aspect | Details |
|--------|---------|
| Flags | `-s` (symbolic), `-f` (force), `-i` (interactive), `-v` (verbose), `-r` (relative), `-n` (no deref), `-b` (backup), `-T` (no target dir) |
| Arguments | TARGET [LINK_NAME] or multiple targets + directory |
| Logic | Hard/symbolic link creation, relative path computation |

### 3.15 `id`

**What it does:** Print user and group IDs.

| Aspect | Details |
|--------|---------|
| Flags | `-u` (user), `-g` (group), `-G` (all groups), `-n` (name), `-r` (real), `-z` (NUL-delimited) |
| Exclusive groups | `-u`, `-g`, `-G` are mutually exclusive |
| Arguments | Optional username |
| Logic | UID/GID lookup, name resolution |

### 3.16 `groups`

**What it does:** Print group memberships.

| Aspect | Details |
|--------|---------|
| Flags | `--help`, `--version` |
| Arguments | Optional variadic username |
| Logic | Group lookup per user |

---

## Tier 4: Complex (10–25+ flags, algorithmic logic)

These tools have rich flag sets and the implementation requires real algorithms:
searching, formatting, archiving, filesystem traversal, etc.

### 4.1 `ls`

**What it does:** List directory contents.

| Aspect | Details |
|--------|---------|
| Flags | ~50 flags including `-l`, `-a`, `-A`, `-h`, `-R`, `-r`, `-t`, `-S`, `-1`, `-C`, `-x`, `--color`, `-d`, `-F`, `-i`, `-g`, `-o`, `-n`, `-G`, `-p`, `-q`, `--group-directories-first`, `--time`, `--sort`, `--format`, etc. |
| Arguments | Variadic path |
| Logic | File metadata, sorting, column formatting, color, symlink display |
| Note | One of the most flag-heavy coreutils; but each flag is simple |

### 4.2 `stat`

**What it does:** Display detailed file status.

| Aspect | Details |
|--------|---------|
| Flags | `-c FORMAT` (custom format), `-f` (filesystem), `-L` (dereference), `-t` (terse) |
| Arguments | Variadic file |
| Logic | Stat syscall, format string interpretation |
| Note | `-c FORMAT` uses printf-like directives (`%s`, `%U`, etc.) — borderline sub-language |

### 4.3 `date`

**What it does:** Display or set the system date/time.

| Aspect | Details |
|--------|---------|
| Flags | `-d STRING` (display given date), `-f FILE` (dates from file), `-I[FMT]` (ISO-8601), `-R` (RFC 5322), `-u` (UTC), `+FORMAT` (output format) |
| Arguments | Optional `+FORMAT` string |
| Logic | Date parsing, timezone handling, format string interpretation |
| Note | `+FORMAT` is a mini-language (like strftime); `-d` needs a date parser |

### 4.4 `grep`

**What it does:** Search for patterns in files.

| Aspect | Details |
|--------|---------|
| Flags | `-E` (extended regex), `-F` (fixed string), `-P` (Perl regex), `-i` (ignore case), `-v` (invert), `-c` (count), `-l` (files only), `-L` (non-matching files), `-n` (line numbers), `-H` (filename), `-h` (no filename), `-r` (recursive), `-w` (word), `-x` (whole line), `-A N` (after), `-B N` (before), `-C N` (context), `-o` (only matching), `-q` (quiet), `--include`, `--exclude`, `--color` |
| Exclusive groups | `-E`, `-F`, `-P` are mutually exclusive |
| Arguments | PATTERN + variadic file |
| Logic | Regex matching, context tracking, recursive directory search |
| Note | The pattern itself is data (not parsed by CLI Builder), so this is fully in scope |

### 4.5 `diff`

**What it does:** Compare files line by line.

| Aspect | Details |
|--------|---------|
| Flags | `-u` (unified), `-c` (context), `-y` (side-by-side), `-r` (recursive), `-q` (brief), `-s` (report same), `-i` (ignore case), `-b` (ignore space changes), `-w` (ignore all space), `-B` (ignore blank lines), `-N` (treat absent as empty), `--color`, `-x PATTERN` (exclude) |
| Arguments | 2 files or directories |
| Logic | LCS/edit-distance algorithm, output formatting |

### 4.6 `xargs`

**What it does:** Build and execute commands from stdin.

| Aspect | Details |
|--------|---------|
| Flags | `-0` (NUL-delimited), `-I REPLACE` (replace string), `-n MAX` (max args), `-P MAX` (parallel), `-t` (verbose), `-p` (prompt), `-r` (no empty run), `-L N` (max lines), `-d DELIM` |
| Arguments | Optional command + args |
| Logic | Stdin tokenization, command building, process execution, parallelism |

### 4.7 `du`

**What it does:** Estimate file space usage.

| Aspect | Details |
|--------|---------|
| Flags | `-a` (all files), `-s` (summary), `-h` (human-readable), `-c` (grand total), `-d N` (max depth), `-k` (1K blocks), `-m` (1M blocks), `-S` (separate dirs), `--apparent-size`, `-x` (one filesystem), `--exclude` |
| Arguments | Variadic path |
| Logic | Recursive directory traversal, size accumulation |

### 4.8 `df`

**What it does:** Report filesystem disk space.

| Aspect | Details |
|--------|---------|
| Flags | `-a` (all), `-h` (human), `-H` (SI), `-i` (inodes), `-k` (1K), `-l` (local), `-P` (POSIX output), `-T` (type), `-t TYPE` (filter type), `-x TYPE` (exclude type) |
| Arguments | Optional variadic filesystem/file |
| Logic | Filesystem stat calls, formatting |

### 4.9 `tar`

**What it does:** Archive files.

| Aspect | Details |
|--------|---------|
| Flags | `-c` (create), `-x` (extract), `-t` (list), `-v` (verbose), `-f FILE` (archive), `-z` (gzip), `-j` (bzip2), `-J` (xz), `-C DIR` (change dir), `-p` (preserve perms), `--exclude`, `--strip-components` |
| Parsing mode | `traditional` (tar-style: `tar xvf` without leading `-`) |
| Arguments | Variadic file |
| Logic | Archive format read/write, compression, directory traversal |
| Note | Great test of CLI Builder's `traditional` parsing mode |

### 4.10 `chmod`

**What it does:** Change file permissions.

| Aspect | Details |
|--------|---------|
| Flags | `-R` (recursive), `-v` (verbose), `-c` (report changes), `-f` (silent), `--preserve-root` |
| Arguments | MODE + variadic file |
| Logic | Symbolic mode parsing (`u+rwx,g-w,o=r`) is a mini-language |
| Note | The MODE argument is borderline — symbolic modes have their own grammar. Octal modes (0755) are simple. Could support octal-only initially. |

### 4.11 `chown` / `chgrp`

**What it does:** Change file ownership.

| Aspect | Details |
|--------|---------|
| Flags | `-R` (recursive), `-v` (verbose), `-c`, `-f`, `--dereference`, `--no-dereference`, `--from`, `--preserve-root` |
| Arguments | OWNER[:GROUP] + variadic file |
| Logic | User/group lookup, recursive traversal, ownership parsing |

---

## Tier 5: Advanced (out of scope or requiring extensions)

These tools have embedded sub-languages or interactive modes that fall outside
CLI Builder's current scope per §1.3.

### 5.1 `find` — Predicate expression tree

The `-name`, `-type`, `-size`, `-newer`, `-exec`, `-and`, `-or`, `-not`
predicates form a full expression language. CLI Builder parses the top-level
flags but cannot parse the predicate tree.

**Could build:** A simplified version that supports a fixed set of filter
flags (`--name`, `--type`, `--size`, `--max-depth`) without the expression
tree semantics.

### 5.2 `sed` — Stream editing language

The `-e SCRIPT` and inline scripts are a complete transformation language
with addresses, commands, hold/pattern space, and branching.

**Out of scope.** The flag parsing (`-e`, `-f`, `-n`, `-i`) is trivial;
the script interpreter is a separate project.

### 5.3 `awk` — Programming language

AWK is a full programming language with pattern-action rules, variables,
functions, and I/O.

**Out of scope.** Like sed, flag parsing is trivial but the interpreter
is a major project.

### 5.4 `less` / `more` — Interactive pager

Terminal UI with keystroke commands, search, screen management.

**Out of scope.** CLI Builder handles the initial flags, but the
interactive mode is a TUI, not CLI parsing.

### 5.5 `printf`

**What it does:** Format and print data.

| Aspect | Details |
|--------|---------|
| Flags | None (GNU printf has `--help`, `--version` only) |
| Arguments | FORMAT + variadic values |
| Logic | Full printf format string interpretation |
| Note | The FORMAT string is the entire program — borderline sub-language |

---

## Recommended Implementation Order

Based on the tiers above, here's a suggested roadmap that builds skills
incrementally:

### Phase 1: Foundation (Tier 1 — validate the pattern)

| # | Tool | Key learning |
|---|------|-------------|
| 1 | `pwd` | **Done.** Established the pattern. |
| 2 | `true` / `false` | Absolute minimum: no flags, no args, just exit code |
| 3 | `echo` | Variadic args, escape interpretation |
| 4 | `yes` | Optional variadic args, infinite output |
| 5 | `whoami` | System info lookup |
| 6 | `sleep` | Required numeric argument |
| 7 | `nproc` | Integer flag with default |
| 8 | `tty` | Silent mode flag |

### Phase 2: File I/O (Tier 2 — read/write files)

| # | Tool | Key learning |
|---|------|-------------|
| 9 | `cat` | Variadic files, stdin fallback, line numbering |
| 10 | `head` | Numeric flag, multi-file headers |
| 11 | `tail` | Same as head but from end (defer `-f` to v2) |
| 12 | `wc` | Multiple counting modes, columnar output |
| 13 | `tee` | Stdin duplication to files |
| 14 | `basename` / `dirname` | Path manipulation |
| 15 | `realpath` / `readlink` | Symlink resolution |

### Phase 3: File Management (Tier 2–3 — create/delete/copy)

| # | Tool | Key learning |
|---|------|-------------|
| 16 | `touch` | Timestamp manipulation |
| 17 | `mkdir` / `rmdir` | Directory creation/removal |
| 18 | `rm` | Recursive deletion, safety prompts |
| 19 | `cp` | Recursive copy, trailing destination pattern |
| 20 | `mv` | Rename or cross-device copy |
| 21 | `ln` | Hard and symbolic links |

### Phase 4: Text Processing (Tier 3 — string/line algorithms)

| # | Tool | Key learning |
|---|------|-------------|
| 22 | `sort` | Multi-key sorting with custom comparators |
| 23 | `uniq` | Adjacent deduplication |
| 24 | `cut` | Mutually exclusive flag groups, range parsing |
| 25 | `paste` | Multi-file line merging |
| 26 | `tr` | Character translation, range expansion |
| 27 | `comm` | Two-file merge comparison |
| 28 | `fold` | Line wrapping |
| 29 | `expand` / `unexpand` | Tab/space conversion |
| 30 | `nl` | Line numbering with sections |
| 31 | `seq` | Number sequence generation |
| 32 | `printenv` / `env` | Environment variable manipulation |

### Phase 5: System & Search (Tier 3–4 — richer logic)

| # | Tool | Key learning |
|---|------|-------------|
| 33 | `id` / `groups` | UID/GID lookup, exclusive groups |
| 34 | `uname` | System info flags |
| 35 | `df` / `du` | Filesystem stats, recursive traversal |
| 36 | `stat` | File metadata, format strings |
| 37 | `md5sum` / `sha256sum` | Hash computation and verification |
| 38 | `grep` | Regex matching, context, recursion |
| 39 | `diff` | Line comparison algorithm |
| 40 | `xargs` | Command building from stdin |

### Phase 6: Power Tools (Tier 4 — complex logic)

| # | Tool | Key learning |
|---|------|-------------|
| 41 | `ls` | The flag-heaviest coreutil (~50 flags) |
| 42 | `tar` | Traditional parsing mode, archive format |
| 43 | `chmod` / `chown` / `chgrp` | Permission/ownership with symbolic modes |
| 44 | `date` | Date parsing and formatting |

---

## Summary Statistics

| Category | Count | Status |
|----------|-------|--------|
| Tier 1 (trivial) | 8 tools | 1 done (pwd) |
| Tier 2 (simple) | 15 tools | Ready to build |
| Tier 3 (moderate) | 16 tools | Ready to build |
| Tier 4 (complex) | 11 tools | Ready to build |
| Tier 5 (out of scope) | 5 tools | Deferred or simplified |
| **Total buildable** | **~50 tools** | |

Each tool built in all 6 languages = **~300 programs** total, all powered by
the same CLI Builder library, all driven by shared JSON spec files.
