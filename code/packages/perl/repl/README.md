# CodingAdventures::Repl

A REPL (Read-Eval-Print Loop) framework for Perl with pluggable Language, Prompt, and Waiting interfaces.

## What is a REPL?

A REPL is the interactive shell mode that most programming languages provide. You type an expression, the language evaluates it, prints the result, and waits for the next expression. Famous examples include Python's `>>>` shell, Ruby's `irb`, Node.js's `node` shell, and Haskell's `ghci`.

This package provides the loop machinery so you can focus on implementing the language-specific eval logic.

## Architecture

The framework separates concerns into three pluggable interfaces:

| Interface | Responsibility | Built-in |
|-----------|---------------|---------|
| `Language` | Evaluate one expression | `EchoLanguage` |
| `Prompt` | Provide prompt strings | `DefaultPrompt` |
| `Waiting` | Show progress during eval | `SilentWaiting` |

### The Language Interface

Implement one method:

```perl
sub eval {
    my ($self, $input) = @_;
    return 'quit'           # exit the REPL
        || ['ok', $output]  # success with output string (or undef)
        || ['error', $msg]; # failure with error message
}
```

### The Prompt Interface

Implement two methods:

```perl
sub global_prompt { return '> '   }   # primary prompt
sub line_prompt   { return '... ' }   # continuation prompt
```

### The Waiting Interface

Implement four methods:

```perl
sub start   { return $state }         # called before eval
sub tick    { my ($self, $s) = @_; return $s }  # called during wait
sub tick_ms { return 100   }          # poll interval in ms
sub stop    { my ($self, $s) = @_ }   # called after eval
```

## Usage

### Simplest case (reads STDIN, writes STDOUT)

```perl
use CodingAdventures::Repl;
use CodingAdventures::Repl::EchoLanguage;

CodingAdventures::Repl::run(
    language => CodingAdventures::Repl::EchoLanguage->new(),
);
```

### With injected I/O (great for testing)

```perl
use CodingAdventures::Repl;
use CodingAdventures::Repl::EchoLanguage;

my @lines  = ('hello', ':quit');
my @output;

CodingAdventures::Repl::run_with_io(
    language  => CodingAdventures::Repl::EchoLanguage->new(),
    input_fn  => sub { shift @lines },
    output_fn => sub { push @output, $_[0] },
);

# @output now contains: ('> ', "hello\n", '> ')
```

### Custom language implementation

```perl
package MyLanguage;

sub new { bless {}, shift }

sub eval {
    my ($self, $input) = @_;
    return 'quit'               if $input eq ':quit';
    return ['error', 'bad cmd'] if $input =~ /^!/;
    return ['ok', uc($input)];  # uppercase everything
}

package main;
use CodingAdventures::Repl;

CodingAdventures::Repl::run(language => MyLanguage->new());
```

## Built-in Implementations

### EchoLanguage

The simplest possible Language. `:quit` exits; everything else is echoed back.

### DefaultPrompt

Classic Unix-style prompts: `"> "` for new expressions, `"... "` for continuation lines.

### SilentWaiting

A no-op Waiting implementation (Null Object pattern). Does nothing. Use it for tests, piped output, or any case where progress display is unnecessary.

## Synchronous Evaluation

Eval is synchronous. The `Waiting` handler brackets each eval call with `start()` and `stop()`, but `tick()` is never called during synchronous eval (the eval blocks the main thread).

This is the same trade-off made by Lua, Python's built-in REPL, and most other simple REPLs. The user can press Ctrl-C (SIGINT) to interrupt an infinite loop.

Perl threads are not used because they are not universally available and introduce significant complexity. See `lib/CodingAdventures/Repl/Waiting.pm` for the full rationale.

## Exception Safety

Every call to `$language->eval()` is wrapped in Perl's `eval {}`:

```perl
my $result = eval { $language->eval($input) };
if ($@) {
    $result = ['error', $@];
}
```

A `die()` inside your language's eval method will NOT crash the REPL. The error is shown to the user and the loop continues.

## Stack Position

This package is a standalone REPL framework with no dependencies on other packages in this codebase. It uses only core Perl modules (`strict`, `warnings`, `Carp`).

## Running the Tests

```bash
cpanm --installdeps .
prove -l -v t/
```

## License

MIT
