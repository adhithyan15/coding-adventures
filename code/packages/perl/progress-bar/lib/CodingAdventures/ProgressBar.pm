package CodingAdventures::ProgressBar;

# ============================================================================
# CodingAdventures::ProgressBar — Pure-Perl text-mode progress bar
# ============================================================================
#
# This module renders a text-based progress bar to a filehandle (or any
# object that supports a print() method). It tracks operations that are
# started, finished, or skipped, and renders a live progress indicator.
#
# The postal worker analogy
# =========================
#
# Imagine a post office with a clerk (the Tracker) and a scoreboard
# (the progress bar). Workers walk up one at a time and hand over a slip
# (an event). The clerk updates the scoreboard immediately.
#
# Unlike goroutine-based Go implementations, this Perl version is fully
# synchronous. Each call to send() immediately updates state and redraws.
# This is the right design for single-threaded Perl.
#
# Visual format
# =============
#
# Flat mode:
#
#   [########............]  7/21  Building: pkg-a, pkg-b  (12.3s)
#
# Labeled flat mode:
#
#   Level 2/3  [####................]  5/12  Building: pkg-a  (8.2s)
#
# Hierarchical (child tracker):
#
#   Level 2/3  [####................]  5/12  Building: pkg-a  (8.2s)
#
# The bar uses Unicode block characters:
#   U+2588 (FULL BLOCK)   — filled portion:  ██████
#   U+2591 (LIGHT SHADE)  — empty portion:   ░░░░░░
#
# We use \r (carriage return) to overwrite the current line in the terminal.
# This works across platforms — no ANSI escape codes needed.
#
# This is a Perl port of the Lua and Go progress-bar implementations in the
# coding-adventures monorepo.

use strict;
use warnings;
use POSIX qw();    # For time() if needed

our $VERSION = '0.01';

# ============================================================================
# Event type constants
# ============================================================================
#
# Three things can happen to a tracked item:
#
#   STARTED  — the item began processing (now "in-flight")
#   FINISHED — the item completed (success or failure)
#   SKIPPED  — the item was bypassed without processing
#
# Using string constants makes debugging easier: "STARTED" is more readable
# than 0 in a stack trace.

use constant STARTED  => 'STARTED';
use constant FINISHED => 'FINISHED';
use constant SKIPPED  => 'SKIPPED';

# Export the constants for callers
our @EXPORT_OK = qw(STARTED FINISHED SKIPPED);

# ============================================================================
# Tracker — the progress bar engine
# ============================================================================
#
# The Tracker is the main object. It receives events and renders a text-based
# progress bar. State is tracked as:
#
#   completed — count of items that are FINISHED or SKIPPED
#   building  — hashref of item names currently in-flight (STARTED not FINISHED)
#   total     — the target count (set at creation time)
#
# Truth table for state transitions on send():
#
#   Event     | completed | building
#   ----------+-----------+---------
#   STARTED   | unchanged | add name
#   FINISHED  | +1        | remove name
#   SKIPPED   | +1        | unchanged (item was never started)

# new($total, $writer, $label)
#
# Creates a new Tracker.
#
# Parameters:
#   $total  — how many items to track (integer > 0)
#   $writer — a filehandle or object with a print() or write() method
#   $label  — optional prefix label ("" or undef for flat/unlabeled mode)
#
# The $writer can be:
#   - A Perl filehandle (STDERR, \*STDERR, an IO::Handle)
#   - Any object with a print() method
#   - A scalar ref (used in tests to capture output)
#
# Returns: a new Tracker object
sub new {
    my ($class, $total, $writer, $label) = @_;

    return bless {
        total      => $total,
        completed  => 0,
        building   => {},        # name => 1 for in-flight items
        writer     => $writer,
        label      => defined($label) ? $label : '',
        start_time => undef,     # set by start()
        parent     => undef,     # set by child() on the child object
        _started   => 0,         # has start() been called?
    }, $class;
}

# start()
#
# Initializes the tracker and records the start time.
# Call this once before sending any events.
#
# In Go, Start() launches a background goroutine. Here, it simply
# records the timestamp. All rendering happens synchronously in send().
sub start {
    my ($self) = @_;
    $self->{start_time} = time();
    $self->{_started}   = 1;
    return;
}

# send(\%event)
#
# Submits an event to the tracker. Immediately updates state and redraws.
#
# If start() hasn't been called, send() is a no-op. This matches the Go
# version: callers can unconditionally call send() without checking state.
#
# The event hashref has these fields:
#   type   — one of STARTED, FINISHED, SKIPPED
#   name   — human-readable identifier (e.g., "python/logic-gates")
#   status — outcome label, meaningful for FINISHED (e.g., "built", "failed")
sub send {
    my ($self, $event) = @_;
    return unless $self->{_started};

    my $type = $event->{type};

    # Update state based on event type.
    if ($type eq STARTED) {
        $self->{building}{$event->{name}} = 1;
    }
    elsif ($type eq FINISHED) {
        delete $self->{building}{$event->{name}};
        $self->{completed}++;
    }
    elsif ($type eq SKIPPED) {
        $self->{completed}++;
    }

    $self->_draw;
    return;
}

# child($total, $label)
#
# Creates a nested sub-tracker for hierarchical progress.
#
# The child shares the parent's writer and start time. When the child
# calls finish(), it advances the parent's completed count by 1.
#
# Example: a build system has 3 dependency levels, each with N packages.
# The parent tracks levels (total=3, label="Level"), and each child
# tracks packages within that level.
#
#   my $parent = CodingAdventures::ProgressBar->new(3, \*STDERR, "Level");
#   $parent->start;
#   my $child = $parent->child(7, "Package");
#   $child->send({ type => STARTED,  name => "pkg-a" });
#   $child->send({ type => FINISHED, name => "pkg-a", status => "built" });
#   $child->finish;   # advances parent by 1
#   $parent->stop;
sub child {
    my ($self, $total, $label) = @_;

    my $c = CodingAdventures::ProgressBar->new($total, $self->{writer}, $label);
    $c->{start_time} = $self->{start_time};
    $c->{parent}     = $self;
    $c->{_started}   = 1;    # Child is immediately active
    return $c;
}

# finish()
#
# Marks this child tracker as complete and advances the parent by one.
# Call this when all items in the child are done.
sub finish {
    my ($self) = @_;

    $self->_draw;   # Final draw showing completed state

    # Notify parent that this child is done
    if (defined $self->{parent}) {
        $self->{parent}->send({
            type => FINISHED,
            name => $self->{label},
        });
    }
    return;
}

# stop()
#
# Shuts down the tracker. Performs a final draw and writes a newline so
# the last progress line is preserved in terminal scrollback.
sub stop {
    my ($self) = @_;
    $self->_draw;
    $self->_write("\n");
    return;
}

# ============================================================================
# Internal: rendering
# ============================================================================

# _draw()
#
# Composes and writes one progress line to the writer.
#
# Bar calculation:
#   bar_width = 20 characters
#   filled = floor(completed * 20 / total)
#   Each filled character = one FULL BLOCK (U+2588)
#   Each empty character  = one LIGHT SHADE (U+2591)
#
# The bar only shows 100% when all items are truly complete (floor rounding).
sub _draw {
    my ($self) = @_;

    my $elapsed = 0.0;
    if (defined $self->{start_time}) {
        $elapsed = time() - $self->{start_time};
    }

    # Build the progress bar string
    my $bar_width = 20;
    my $filled    = 0;
    if ($self->{total} > 0) {
        $filled = int($self->{completed} * $bar_width / $self->{total});
    }
    $filled = $bar_width if $filled > $bar_width;

    # Unicode block characters: U+2588 = full block, U+2591 = light shade
    my $bar = ("\x{2588}" x $filled) . ("\x{2591}" x ($bar_width - $filled));

    # Build the activity string (what's currently being worked on)
    my $activity = _format_activity(
        $self->{building},
        $self->{completed},
        $self->{total}
    );

    # Compose the line based on whether we have a parent (hierarchical mode)
    my $line;
    if (defined $self->{parent}) {
        # Hierarchical: show parent label and count (+1 because this child IS the current one)
        my $parent_completed = $self->{parent}{completed} + 1;
        $line = sprintf("\r%s %d/%d  [%s]  %d/%d  %s  (%.1fs)",
            $self->{parent}{label},
            $parent_completed,
            $self->{parent}{total},
            $bar,
            $self->{completed},
            $self->{total},
            $activity,
            $elapsed
        );
    }
    elsif ($self->{label} ne '') {
        # Labeled flat tracker
        $line = sprintf("\r%s %d/%d  [%s]  %s  (%.1fs)",
            $self->{label},
            $self->{completed},
            $self->{total},
            $bar,
            $activity,
            $elapsed
        );
    }
    else {
        # Flat/unlabeled mode
        $line = sprintf("\r[%s]  %d/%d  %s  (%.1fs)",
            $bar,
            $self->{completed},
            $self->{total},
            $activity,
            $elapsed
        );
    }

    # Pad to 80 chars to overwrite any previous longer line
    $self->_write(sprintf("%-80s", $line));
    return;
}

# _write($str)
#
# Writes a string to the writer. Handles filehandles, objects with
# print() methods, and scalar refs (for testing).
sub _write {
    my ($self, $str) = @_;
    my $w = $self->{writer};

    if (ref($w) eq 'SCALAR') {
        # Test mode: append to a string
        $$w .= $str;
    }
    elsif (ref($w) && $w->can('write')) {
        $w->write($str);
    }
    elsif (ref($w) && $w->can('print')) {
        $w->print($str);
    }
    else {
        # Assume it's a filehandle glob
        print $w $str;
    }
    return;
}

# ============================================================================
# _format_activity($building, $completed, $total)
# ============================================================================
#
# Builds the activity string showing what's currently being processed.
#
# Rules:
#
#   | In-flight count | Completed vs Total  | Output                       |
#   |-----------------|---------------------|------------------------------|
#   | 0               | completed < total   | "waiting..."                 |
#   | 0               | completed >= total  | "done"                       |
#   | 1-3             | any                 | "Building: a, b, c"          |
#   | 4+              | any                 | "Building: a, b, c +N more"  |
#
# Names are sorted alphabetically for deterministic output.
sub _format_activity {
    my ($building, $completed, $total) = @_;

    my @names = sort keys %$building;

    if (@names == 0) {
        return $completed >= $total ? 'done' : 'waiting...';
    }

    my $max_names = 3;
    if (@names <= $max_names) {
        return 'Building: ' . join(', ', @names);
    }

    # Show first 3 names plus "+N more" suffix
    my @shown = @names[0 .. $max_names - 1];
    my $extra = @names - $max_names;
    return sprintf('Building: %s +%d more', join(', ', @shown), $extra);
}

# ============================================================================
# percentage($current, $total)
# ============================================================================
#
# Computes the percentage completion as an integer (0–100).
# Returns 0 if $total is 0 (avoids division by zero).
#
# Parameters:
#   $current — items completed so far
#   $total   — total items
sub percentage {
    my ($class_or_self, $current, $total) = @_;
    # Handle both class method and object method calls
    if (ref $class_or_self) {
        # Called as object method without args: use own state
        unless (defined $current) {
            ($current, $total) = ($class_or_self->{completed}, $class_or_self->{total});
        }
    }
    return 0 if !defined($total) || $total == 0;
    my $pct = int($current * 100 / $total);
    return $pct > 100 ? 100 : $pct;
}

# ============================================================================
# bar_string($pct, $width, $fill_char, $empty_char)
# ============================================================================
#
# Returns a progress bar string of the given width.
#
# Parameters:
#   $pct        — percentage (0–100)
#   $width      — total bar width in characters
#   $fill_char  — character for filled portion (default: U+2588 full block)
#   $empty_char — character for empty portion (default: U+2591 light shade)
#
# Example:
#   bar_string(50, 10, '#', '.')  =>  "#####....."
sub bar_string {
    my ($class_or_self, $pct, $width, $fill_char, $empty_char) = @_;

    # Handle both class and object call
    unless (defined $width) {
        # Called as object method: shift the args
        ($pct, $width, $fill_char, $empty_char) =
            ($class_or_self, $pct, $width, $fill_char);
    }

    $pct        //= 0;
    $width      //= 20;
    $fill_char  //= "\x{2588}";
    $empty_char //= "\x{2591}";

    $pct = 0   if $pct < 0;
    $pct = 100 if $pct > 100;

    my $filled = int($pct * $width / 100);
    return ($fill_char x $filled) . ($empty_char x ($width - $filled));
}

# Make _format_activity accessible for testing
our $FORMAT_ACTIVITY = \&_format_activity;

1;

__END__

=head1 NAME

CodingAdventures::ProgressBar - Pure-Perl text-mode progress bar renderer

=head1 SYNOPSIS

    use CodingAdventures::ProgressBar;

    # Simple flat tracker
    my $t = CodingAdventures::ProgressBar->new(10, \*STDERR, '');
    $t->start;
    $t->send({ type => CodingAdventures::ProgressBar::STARTED,  name => 'pkg-a' });
    $t->send({ type => CodingAdventures::ProgressBar::FINISHED, name => 'pkg-a', status => 'built' });
    $t->stop;

    # Percentage and bar string utilities
    my $pct = CodingAdventures::ProgressBar->percentage(7, 21);  # 33
    my $bar = CodingAdventures::ProgressBar->bar_string(50, 10, '#', '.');  # "#####....."

=head1 DESCRIPTION

Renders a text-based progress bar in the terminal. Tracks three event types:
STARTED (item in-flight), FINISHED (item done), SKIPPED (item bypassed).
Supports flat mode and hierarchical (parent/child) mode for multi-level builds.

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
