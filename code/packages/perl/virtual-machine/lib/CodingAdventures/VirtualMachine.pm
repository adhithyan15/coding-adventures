package CodingAdventures::VirtualMachine;

# ============================================================================
# CodingAdventures::VirtualMachine — Pure-Perl Stack-Based Bytecode VM
# ============================================================================
#
# This module implements a simple stack-based virtual machine (VM) — a
# software CPU that executes bytecode programs.
#
# === WHAT IS A VIRTUAL MACHINE? ===
#
# A virtual machine is a software CPU. Real CPUs execute machine code (raw
# binary instructions). A VM does the same thing, but the "machine code" is
# a sequence of Perl data structures instead of raw bytes, and the "CPU
# registers" are Perl variables.
#
# Why bother? Because a VM lets you run programs written in any language —
# Python, Ruby, your own invention — as long as you have a compiler that
# translates source code into the VM's instruction set (bytecode).
#
# === ARCHITECTURE OVERVIEW ===
#
#   +---------------------------------------------------+
#   |                  VirtualMachine                    |
#   |                                                    |
#   |   +---------+   +-----------+   +--------+        |
#   |   |  Stack  |   | Variables |   | Locals |        |
#   |   | (LIFO)  |   |  (hash)   |   | (array)|        |
#   |   +---------+   +-----------+   +--------+        |
#   |                                                    |
#   |   PC --> fetch instruction --> dispatch by opcode  |
#   |          --> modify state --> record trace          |
#   +---------------------------------------------------+
#
# === THE EXECUTION CYCLE (Fetch-Decode-Execute) ===
#
# Every real CPU and every virtual machine follows the same basic loop:
#
#   1. FETCH:   Read the instruction at the current Program Counter (PC).
#   2. DECODE:  Figure out what the instruction means (look at opcode).
#   3. EXECUTE: Do the work (modify VM state: push, pop, jump, etc.).
#   4. REPEAT:  Go back to step 1 unless halted.
#
# === OPCODE CONSTANTS ===
#
# Each opcode is a numeric constant, grouped by category:
#
#   0x01-0x03   Stack:      LOAD_CONST, POP, DUP
#   0x10-0x13   Variables:  STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL
#   0x20-0x23   Arithmetic: ADD, SUB, MUL, DIV
#   0x30-0x32   Comparison: CMP_EQ, CMP_LT, CMP_GT
#   0x40-0x42   Control:    JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE
#   0x50-0x51   Functions:  CALL, RETURN
#   0x60        I/O:        PRINT
#   0xFF        Control:    HALT
#
# === STACK SEMANTICS ===
#
# The stack is a Last-In-First-Out (LIFO) structure, like a stack of plates.
# Most arithmetic instructions "pop" two values, perform the operation, and
# "push" the result. For example, ADD:
#
#   Stack before: [10, 20]   (20 is on top)
#   ADD: pop 20, pop 10, push (10 + 20)
#   Stack after:  [30]
#
# Note that we push the LEFT operand first, then the RIGHT. So when we pop:
#   b = pop()   # right operand (was pushed last)
#   a = pop()   # left operand (was pushed first)
#   result = a OP b
#
# === FALSINESS ===
#
# For conditional jumps (JUMP_IF_FALSE, JUMP_IF_TRUE), the VM uses C/Python
# style falsiness:
#
#   Value        | Falsy?
#   -------------|-------
#   undef        | YES
#   0            | YES
#   ""           | YES
#   1            | no
#   -1           | no
#   "hello"      | no
#   any ref      | no
#
# This differs from Perl's own truthiness (where "0" is also false).
#
# === USAGE EXAMPLE ===
#
#   use CodingAdventures::VirtualMachine;
#
#   # Create a VM
#   my $vm = CodingAdventures::VirtualMachine->new();
#
#   # Build a program: push 10, push 20, add, print, halt
#   my $code = CodingAdventures::VirtualMachine::CodeObject->new(
#       instructions => [
#           { opcode => 0x01, operand => 0 },  # LOAD_CONST constants[0]=10
#           { opcode => 0x01, operand => 1 },  # LOAD_CONST constants[1]=20
#           { opcode => 0x20 },                # ADD
#           { opcode => 0x60 },                # PRINT
#           { opcode => 0xFF },                # HALT
#       ],
#       constants => [10, 20],
#   );
#
#   my $traces = $vm->execute($code);
#   print $vm->output->[0];  # "30"
#
# ============================================================================

use strict;
use warnings;

our $VERSION = '0.01';

# ============================================================================
# ERROR CLASSES
# ============================================================================
#
# We define lightweight error packages so callers can catch specific
# error types with eval/die.

package CodingAdventures::VirtualMachine::Error;
sub new {
    my ($class, $message) = @_;
    return bless { message => $message }, $class;
}
sub message { return $_[0]->{message} }
sub throw {
    my ($class, $message) = @_;
    die $class->new($message);
}

package CodingAdventures::VirtualMachine::StackUnderflowError;
use parent -norequire, 'CodingAdventures::VirtualMachine::Error';

package CodingAdventures::VirtualMachine::InvalidOpcodeError;
use parent -norequire, 'CodingAdventures::VirtualMachine::Error';

package CodingAdventures::VirtualMachine::InvalidOperandError;
use parent -norequire, 'CodingAdventures::VirtualMachine::Error';

package CodingAdventures::VirtualMachine::UndefinedNameError;
use parent -norequire, 'CodingAdventures::VirtualMachine::Error';

package CodingAdventures::VirtualMachine::DivisionByZeroError;
use parent -norequire, 'CodingAdventures::VirtualMachine::Error';

# ============================================================================
# OPCODE CONSTANTS
# ============================================================================

package CodingAdventures::VirtualMachine::OpCode;

# Stack operations
use constant LOAD_CONST    => 0x01;  # Push a constant from the constants pool
use constant POP           => 0x02;  # Discard the top of stack
use constant DUP           => 0x03;  # Duplicate the top of stack

# Variable operations
use constant STORE_NAME    => 0x10;  # Pop and store in a named variable
use constant LOAD_NAME     => 0x11;  # Push a named variable's value
use constant STORE_LOCAL   => 0x12;  # Pop and store in a local slot
use constant LOAD_LOCAL    => 0x13;  # Push a local slot's value

# Arithmetic operations
use constant ADD           => 0x20;  # Pop two, push sum
use constant SUB           => 0x21;  # Pop two, push difference (a - b)
use constant MUL           => 0x22;  # Pop two, push product
use constant DIV           => 0x23;  # Pop two, push quotient

# Comparison operations (result is 1 or 0)
use constant CMP_EQ        => 0x30;  # Pop two, push 1 if equal, 0 otherwise
use constant CMP_LT        => 0x31;  # Pop two, push 1 if a < b, 0 otherwise
use constant CMP_GT        => 0x32;  # Pop two, push 1 if a > b, 0 otherwise

# Control flow
use constant JUMP          => 0x40;  # Unconditional jump to operand (0-based index)
use constant JUMP_IF_FALSE => 0x41;  # Pop; jump if falsy (0, undef, "")
use constant JUMP_IF_TRUE  => 0x42;  # Pop; jump if truthy

# Function operations
use constant CALL          => 0x50;  # Call a function by name
use constant RETURN        => 0x51;  # Return from a function

# I/O
use constant PRINT         => 0x60;  # Pop and print to output

# VM control
use constant HALT          => 0xFF;  # Stop execution

# Map opcode integers to human-readable names for error messages and traces.
use constant NAMES => {
    LOAD_CONST()    => 'LOAD_CONST',
    POP()           => 'POP',
    DUP()           => 'DUP',
    STORE_NAME()    => 'STORE_NAME',
    LOAD_NAME()     => 'LOAD_NAME',
    STORE_LOCAL()   => 'STORE_LOCAL',
    LOAD_LOCAL()    => 'LOAD_LOCAL',
    ADD()           => 'ADD',
    SUB()           => 'SUB',
    MUL()           => 'MUL',
    DIV()           => 'DIV',
    CMP_EQ()        => 'CMP_EQ',
    CMP_LT()        => 'CMP_LT',
    CMP_GT()        => 'CMP_GT',
    JUMP()          => 'JUMP',
    JUMP_IF_FALSE() => 'JUMP_IF_FALSE',
    JUMP_IF_TRUE()  => 'JUMP_IF_TRUE',
    CALL()          => 'CALL',
    RETURN()        => 'RETURN',
    PRINT()         => 'PRINT',
    HALT()          => 'HALT',
};

# ============================================================================
# INSTRUCTION
# ============================================================================
#
# An Instruction is the smallest unit of work: an opcode and an optional
# operand. For example:
#
#   { opcode => 0x01, operand => 0 }  -- LOAD_CONST: push constants[0]
#   { opcode => 0x20 }                -- ADD: pop two, push sum
#   { opcode => 0x40, operand => 5 }  -- JUMP: set PC to 5
#
# We represent instructions as plain Perl hash references. No class needed
# for such a simple structure.

package CodingAdventures::VirtualMachine::Instruction;

sub new {
    my ($class, %args) = @_;
    return bless {
        opcode  => $args{opcode},
        operand => $args{operand},  # may be undef
    }, $class;
}

sub opcode  { return $_[0]->{opcode}  }
sub operand { return $_[0]->{operand} }

# ============================================================================
# CODE OBJECT
# ============================================================================
#
# A CodeObject bundles everything needed to run a bytecode program:
#
#   instructions  -- the sequence of Instruction objects
#   constants     -- the constant pool (numbers, strings, etc.)
#   names         -- the name pool (variable/function name strings)
#
# Think of it like an executable file:
#   instructions = code segment
#   constants    = data segment (read-only literals)
#   names        = symbol table (variable/function names)

package CodingAdventures::VirtualMachine::CodeObject;

sub new {
    my ($class, %args) = @_;
    return bless {
        instructions => $args{instructions} || [],
        constants    => $args{constants}    || [],
        names        => $args{names}        || [],
    }, $class;
}

sub instructions { return $_[0]->{instructions} }
sub constants    { return $_[0]->{constants}    }
sub names        { return $_[0]->{names}        }

# ============================================================================
# CALL FRAME
# ============================================================================
#
# When a CALL instruction executes, we save the caller's context in a
# CallFrame so we can restore it when RETURN executes.
#
# A CallFrame stores:
#   return_address  -- the PC to resume in the caller
#   saved_variables -- a copy of the caller's named variables
#   saved_locals    -- a copy of the caller's local slots

package CodingAdventures::VirtualMachine::CallFrame;

sub new {
    my ($class, %args) = @_;
    return bless {
        return_address  => $args{return_address},
        saved_variables => $args{saved_variables} || {},
        saved_locals    => $args{saved_locals}    || [],
    }, $class;
}

sub return_address  { return $_[0]->{return_address}  }
sub saved_variables { return $_[0]->{saved_variables} }
sub saved_locals    { return $_[0]->{saved_locals}    }

# ============================================================================
# VM TRACE
# ============================================================================
#
# A VMTrace is a snapshot of one instruction's execution, recording:
#
#   pc            -- the program counter BEFORE the instruction ran
#   instruction   -- the instruction that was executed
#   stack_before  -- copy of the stack BEFORE execution
#   stack_after   -- copy of the stack AFTER execution
#   variables     -- copy of the variable environment AFTER execution
#   output        -- string if the instruction produced output, undef otherwise
#   description   -- human-readable explanation of what happened
#
# Traces are invaluable for debugging and education. You can replay a
# program's execution step by step and inspect state at every instruction.

package CodingAdventures::VirtualMachine::VMTrace;

sub new {
    my ($class, %args) = @_;
    return bless {
        pc           => $args{pc},
        instruction  => $args{instruction},
        stack_before => $args{stack_before} || [],
        stack_after  => $args{stack_after}  || [],
        variables    => $args{variables}    || {},
        output       => $args{output},
        description  => $args{description}  || '',
    }, $class;
}

sub pc           { return $_[0]->{pc}           }
sub instruction  { return $_[0]->{instruction}  }
sub stack_before { return $_[0]->{stack_before} }
sub stack_after  { return $_[0]->{stack_after}  }
sub variables    { return $_[0]->{variables}    }
sub output       { return $_[0]->{output}       }
sub description  { return $_[0]->{description}  }

# ============================================================================
# VIRTUAL MACHINE (Main Class)
# ============================================================================
#
# The main VM class. Holds all mutable execution state and implements the
# fetch-decode-execute loop.

package CodingAdventures::VirtualMachine;

sub new {
    my ($class) = @_;
    my $self = bless {}, $class;
    $self->_reset();
    return $self;
}

# ---- State accessors --------------------------------------------------------

sub stack      { return $_[0]->{stack}      }
sub variables  { return $_[0]->{variables}  }
sub locals     { return $_[0]->{locals}     }
sub pc         { return $_[0]->{pc}         }
sub halted     { return $_[0]->{halted}     }
sub output     { return $_[0]->{output}     }
sub call_stack { return $_[0]->{call_stack} }

# registers() returns a summary hash for inspection (mirrors the spec API)
sub registers {
    my ($self) = @_;
    return {
        pc     => $self->{pc},
        halted => $self->{halted},
        stack  => [ @{ $self->{stack} } ],
    };
}

# ---- Lifecycle --------------------------------------------------------------

# Reset the VM to its initial state. Used internally and exposed for reuse.
sub _reset {
    my ($self) = @_;
    $self->{stack}            = [];   # operand stack (LIFO array, top at the end)
    $self->{variables}        = {};   # named variable storage (global scope)
    $self->{locals}           = [];   # indexed local variable slots
    $self->{pc}               = 0;    # program counter (0-based index)
    $self->{halted}           = 0;    # 1 when HALT has been executed
    $self->{output}           = [];   # accumulated PRINT output
    $self->{call_stack}       = [];   # saved CallFrame stack for CALL/RETURN
    $self->{typed_stack}      = [];   # typed value stack: [{type => N, value => V}, ...]
    $self->{_context_handlers} //= {};  # preserve across reset
    $self->{_current_context} = undef;  # execution context for context opcodes
    $self->{_pre_step_hook}   = undef;  # hook called before each instruction
    $self->{_max_recursion}   = 1024;   # maximum call depth
}

# load(program) — reset and prepare to run a new program.
# 'program' should be a CodeObject (or just a hashref with same fields).
sub load {
    my ($self, $program) = @_;
    $self->_reset();
    $self->{_program} = $program;
    return $self;
}

# run() — execute the loaded program to completion, return traces.
# Requires load() to have been called first.
sub run {
    my ($self) = @_;
    my $code = $self->{_program}
        or die "No program loaded — call load() first\n";
    return $self->execute($code);
}

# ---- Main execution ---------------------------------------------------------

# execute(code) — run a CodeObject to completion.
# Returns an arrayref of VMTrace objects.
sub execute {
    my ($self, $code) = @_;
    my @traces;
    my $instrs = $code->instructions();
    while ( !$self->{halted} && $self->{pc} < scalar(@$instrs) ) {
        push @traces, $self->step($code);
    }
    return \@traces;
}

# step(code) — execute one instruction and return a VMTrace.
#
# This is the core of the VM:
#   1. Fetch: read instruction at current PC
#   2. Decode + Execute: dispatch by opcode
#   3. Build trace: record what happened
sub step {
    my ($self, $code) = @_;

    my $instrs = $code->instructions();
    my $instr  = $instrs->[ $self->{pc} ];

    # Snapshot state BEFORE execution
    my $pc_before    = $self->{pc};
    my @stack_before = @{ $self->{stack} };

    # Execute the instruction
    my $output_value = $self->_dispatch($instr, $code);

    # Build human-readable description
    my $description = $self->_describe($instr, $code, \@stack_before);

    return CodingAdventures::VirtualMachine::VMTrace->new(
        pc           => $pc_before,
        instruction  => $instr,
        stack_before => \@stack_before,
        stack_after  => [ @{ $self->{stack} } ],
        variables    => { %{ $self->{variables} } },
        output       => $output_value,
        description  => $description,
    );
}

# ---- Dispatch ---------------------------------------------------------------
#
# _dispatch() is the big switch statement. It reads the opcode and
# performs the corresponding operation on VM state.

sub _dispatch {
    my ($self, $instr, $code) = @_;
    my $opcode  = $instr->{opcode};
    my $operand = $instr->{operand};
    my $output  = undef;

    # --- Context opcode handlers take priority when a context is active ---
    # When executing with a context (e.g., WASM execution), registered context
    # handlers MUST be checked first because WASM opcodes reuse the same numeric
    # values as standard VM opcodes (e.g., WASM nop=0x01 vs VM LOAD_CONST=0x01).
    if ($self->{_current_context} && exists $self->{_context_handlers}{$opcode}) {
        my $handler = $self->{_context_handlers}{$opcode};
        $handler->($self, $instr, $code, $self->{_current_context});
        return $output;
    }

    # --- Stack operations ---

    if ( $opcode == CodingAdventures::VirtualMachine::OpCode::LOAD_CONST ) {
        # Push a constant from the constant pool.
        # The operand is a 0-based index into code->constants.
        my $idx = $self->_require_operand($instr);
        my $consts = $code->constants();
        $self->_validate_index($idx, scalar(@$consts), 'LOAD_CONST', 'constants pool');
        push @{ $self->{stack} }, $consts->[$idx];
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::POP ) {
        # Discard the top of stack.
        $self->_do_pop();
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::DUP ) {
        # Duplicate the top of stack.
        # Before: [a, b, c]
        # After:  [a, b, c, c]
        CodingAdventures::VirtualMachine::StackUnderflowError->throw(
            'DUP requires at least one value on the stack'
        ) if @{ $self->{stack} } == 0;
        push @{ $self->{stack} }, $self->{stack}[-1];
        $self->{pc}++;

    # --- Variable operations ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::STORE_NAME ) {
        # Pop a value and store it under a name from the names pool.
        my $idx  = $self->_require_operand($instr);
        my $names = $code->names();
        $self->_validate_index($idx, scalar(@$names), 'STORE_NAME', 'names pool');
        my $name = $names->[$idx];
        my $val  = $self->_do_pop();
        $self->{variables}{$name} = $val;
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::LOAD_NAME ) {
        # Push the value of a named variable onto the stack.
        my $idx  = $self->_require_operand($instr);
        my $names = $code->names();
        $self->_validate_index($idx, scalar(@$names), 'LOAD_NAME', 'names pool');
        my $name = $names->[$idx];
        unless ( exists $self->{variables}{$name} ) {
            CodingAdventures::VirtualMachine::UndefinedNameError->throw(
                "Variable '$name' is not defined"
            );
        }
        push @{ $self->{stack} }, $self->{variables}{$name};
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::STORE_LOCAL ) {
        # Pop a value and store it in an indexed local slot.
        my $idx = $self->_require_operand($instr);
        CodingAdventures::VirtualMachine::InvalidOperandError->throw(
            "STORE_LOCAL operand must be a non-negative integer, got $idx"
        ) unless defined($idx) && $idx =~ /^\d+$/;
        my $val = $self->_do_pop();
        $self->{locals}[$idx] = $val;
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::LOAD_LOCAL ) {
        # Push the value of a local slot onto the stack.
        my $idx = $self->_require_operand($instr);
        CodingAdventures::VirtualMachine::InvalidOperandError->throw(
            "LOAD_LOCAL operand must be a non-negative integer, got $idx"
        ) unless defined($idx) && $idx =~ /^\d+$/;
        CodingAdventures::VirtualMachine::InvalidOperandError->throw(
            "LOAD_LOCAL slot $idx has not been initialized"
        ) if $idx >= scalar(@{ $self->{locals} });
        push @{ $self->{stack} }, $self->{locals}[$idx];
        $self->{pc}++;

    # --- Arithmetic ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::ADD ) {
        # Pop b, pop a, push (a + b).
        # For numbers: arithmetic addition.
        # For strings: concatenation.
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, $a + $b;
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::SUB ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, $a - $b;
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::MUL ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, $a * $b;
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::DIV ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        CodingAdventures::VirtualMachine::DivisionByZeroError->throw(
            'Division by zero'
        ) if $b == 0;
        # Use integer division like Go/Ruby implementations
        push @{ $self->{stack} }, int($a / $b);
        $self->{pc}++;

    # --- Comparison ---
    #
    # Each comparison pops two values and pushes 1 (true) or 0 (false).

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::CMP_EQ ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, ( $a == $b ? 1 : 0 );
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::CMP_LT ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, ( $a < $b ? 1 : 0 );
        $self->{pc}++;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::CMP_GT ) {
        my $b = $self->_do_pop();
        my $a = $self->_do_pop();
        push @{ $self->{stack} }, ( $a > $b ? 1 : 0 );
        $self->{pc}++;

    # --- Control Flow ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::JUMP ) {
        # Unconditional jump — set PC to operand (0-based).
        my $target = $self->_require_operand($instr);
        $self->{pc} = $target;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::JUMP_IF_FALSE ) {
        # Pop condition; jump if falsy, else fall through.
        my $target    = $self->_require_operand($instr);
        my $condition = $self->_do_pop();
        $self->{pc} = $self->_is_falsy($condition) ? $target : $self->{pc} + 1;

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::JUMP_IF_TRUE ) {
        # Pop condition; jump if truthy, else fall through.
        my $target    = $self->_require_operand($instr);
        my $condition = $self->_do_pop();
        $self->{pc} = $self->_is_falsy($condition) ? $self->{pc} + 1 : $target;

    # --- Functions ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::CALL ) {
        # Call a function stored in variables under names[operand].
        # Saves caller context in call_stack, then executes the callee.
        my $name_idx = $self->_require_operand($instr);
        my $names    = $code->names();
        $self->_validate_index($name_idx, scalar(@$names), 'CALL', 'names pool');
        my $func_name = $names->[$name_idx];

        unless ( exists $self->{variables}{$func_name} ) {
            CodingAdventures::VirtualMachine::UndefinedNameError->throw(
                "Function '$func_name' is not defined"
            );
        }
        my $func_code = $self->{variables}{$func_name};
        unless ( ref($func_code) && $func_code->isa('CodingAdventures::VirtualMachine::CodeObject') ) {
            CodingAdventures::VirtualMachine::Error->throw(
                "'$func_name' is not callable"
            );
        }

        # Save caller state
        my $frame = CodingAdventures::VirtualMachine::CallFrame->new(
            return_address  => $self->{pc} + 1,
            saved_variables => { %{ $self->{variables} } },
            saved_locals    => [ @{ $self->{locals} } ],
        );
        push @{ $self->{call_stack} }, $frame;

        # Execute callee inline
        $self->{locals} = [];
        $self->{pc}     = 0;
        my $func_instrs = $func_code->instructions();
        while ( !$self->{halted} && $self->{pc} < scalar(@$func_instrs) ) {
            my $fi = $func_instrs->[ $self->{pc} ];
            last if $fi->{opcode} == CodingAdventures::VirtualMachine::OpCode::RETURN;
            $self->_dispatch($fi, $func_code);
        }

        # Restore caller state
        my $saved = pop @{ $self->{call_stack} };
        $self->{pc}     = $saved->return_address();
        $self->{locals} = [ @{ $saved->saved_locals() } ];

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::RETURN ) {
        # Return from a function. If no call stack, halt.
        if ( @{ $self->{call_stack} } ) {
            my $frame = pop @{ $self->{call_stack} };
            $self->{pc}     = $frame->return_address();
            $self->{locals} = [ @{ $frame->saved_locals() } ];
        } else {
            $self->{halted} = 1;
        }

    # --- I/O ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::PRINT ) {
        # Pop and record as output. Returns the string for the trace.
        my $value = $self->_do_pop();
        my $str   = defined($value) ? "$value" : '';
        push @{ $self->{output} }, $str;
        $output = $str;
        $self->{pc}++;

    # --- VM Control ---

    } elsif ( $opcode == CodingAdventures::VirtualMachine::OpCode::HALT ) {
        $self->{halted} = 1;

    } else {
        # Check if there's a registered context opcode handler
        if (exists $self->{_context_handlers}{$opcode}) {
            my $handler = $self->{_context_handlers}{$opcode};
            $handler->($self, $instr, $code, $self->{_current_context});
            # Context opcodes manage their own PC advancement
        } else {
            CodingAdventures::VirtualMachine::InvalidOpcodeError->throw(
                sprintf('Unknown opcode: 0x%02x', $opcode)
            );
        }
    }

    return $output;
}

# ---- Helper methods ---------------------------------------------------------

# Pop the top value from the operand stack.
# Dies with StackUnderflowError if the stack is empty.
sub _do_pop {
    my ($self) = @_;
    CodingAdventures::VirtualMachine::StackUnderflowError->throw(
        'Cannot pop from an empty stack — possible compiler bug'
    ) if @{ $self->{stack} } == 0;
    return pop @{ $self->{stack} };
}

# Get operand from instruction, raising InvalidOperandError if absent.
sub _require_operand {
    my ($self, $instr) = @_;
    if ( !defined $instr->{operand} ) {
        my $name = CodingAdventures::VirtualMachine::OpCode::NAMES->{$instr->{opcode}}
            || sprintf('0x%02x', $instr->{opcode});
        CodingAdventures::VirtualMachine::InvalidOperandError->throw(
            "$name requires an operand but none was provided"
        );
    }
    return $instr->{operand};
}

# Validate that an index is in-range for a pool.
sub _validate_index {
    my ($self, $idx, $pool_size, $op_name, $pool_name) = @_;
    unless ( defined($idx) && $idx =~ /^\d+$/ && $idx >= 0 && $idx < $pool_size ) {
        CodingAdventures::VirtualMachine::InvalidOperandError->throw(
            "$op_name operand $idx is out of range ($pool_name has $pool_size entries)"
        );
    }
}

# Test whether a value is "falsy" in VM semantics.
#
# VM falsiness (C/Python style):
#   undef  -> true  (absence of a value)
#   0      -> true  (C-style: zero is false)
#   ""     -> true  (empty string is false)
#   else   -> false (any other value is truthy)
sub _is_falsy {
    my ($self, $val) = @_;
    return 1 if !defined($val);
    return 1 if $val == 0;
    return 1 if $val eq '';
    return 0;
}

# Generate a human-readable description of what an instruction did.
# Used to populate VMTrace->description.
sub _describe {
    my ($self, $instr, $code, $stack_before) = @_;
    my $op  = $instr->{opcode};
    my $opr = $instr->{operand};

    if ( $op == CodingAdventures::VirtualMachine::OpCode::LOAD_CONST ) {
        my $consts = $code->constants();
        my $val = ( defined($opr) && $opr >= 0 && $opr < @$consts ) ? $consts->[$opr] : '?';
        return "Push constant $val onto the stack";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::POP ) {
        my $val = @$stack_before ? $stack_before->[-1] : '?';
        return "Discard top of stack ($val)";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::DUP ) {
        my $val = @$stack_before ? $stack_before->[-1] : '?';
        return "Duplicate top of stack ($val)";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::STORE_NAME ) {
        my $names = $code->names();
        my $name  = ( defined($opr) && $opr >= 0 && $opr < @$names ) ? $names->[$opr] : '?';
        my $val   = @$stack_before ? $stack_before->[-1] : '?';
        return "Store $val into variable '$name'";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::LOAD_NAME ) {
        my $names = $code->names();
        my $name  = ( defined($opr) && $opr >= 0 && $opr < @$names ) ? $names->[$opr] : '?';
        return "Push variable '$name' onto the stack";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::ADD ) {
        if ( @$stack_before >= 2 ) {
            my ($a, $b) = ( $stack_before->[-2], $stack_before->[-1] );
            return "Pop $b and $a, push sum " . ($a + $b);
        }
        return "Add top two stack values";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::SUB ) {
        return "Subtract top two stack values";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::MUL ) {
        return "Multiply top two stack values";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::DIV ) {
        return "Divide top two stack values";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::CMP_EQ ) {
        return "Compare top two stack values for equality";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::CMP_LT ) {
        return "Compare top two stack values (less than)";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::CMP_GT ) {
        return "Compare top two stack values (greater than)";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::JUMP ) {
        return "Jump to instruction $opr";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::JUMP_IF_FALSE ) {
        return "Jump to $opr if top of stack is falsy";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::JUMP_IF_TRUE ) {
        return "Jump to $opr if top of stack is truthy";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::CALL ) {
        my $names = $code->names();
        my $name  = ( defined($opr) && $opr >= 0 && $opr < @$names ) ? $names->[$opr] : '?';
        return "Call function '$name'";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::RETURN ) {
        return "Return from function";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::PRINT ) {
        my $val = @$stack_before ? $stack_before->[-1] : '?';
        return "Print $val";

    } elsif ( $op == CodingAdventures::VirtualMachine::OpCode::HALT ) {
        return "Halt execution";

    } else {
        return sprintf("Unknown operation (0x%02x)", $op);
    }
}

# ============================================================================
# TYPED STACK — Push/pop values tagged with their type
# ============================================================================
#
# The typed stack is parallel to the regular stack but carries type information
# alongside each value. This is essential for WebAssembly execution where every
# value has a type (i32, i64, f32, f64).
#
# A typed value is a hashref: { type => $type_code, value => $raw_value }

sub typed_stack { return $_[0]->{typed_stack} }

# push_typed($typed_value) — push a {type => N, value => V} onto typed stack.
sub push_typed {
    my ($self, $typed_val) = @_;
    push @{ $self->{typed_stack} }, $typed_val;
}

# pop_typed() — pop and return a typed value from the typed stack.
sub pop_typed {
    my ($self) = @_;
    CodingAdventures::VirtualMachine::StackUnderflowError->throw(
        'Cannot pop from an empty typed stack'
    ) if @{ $self->{typed_stack} } == 0;
    return pop @{ $self->{typed_stack} };
}

# peek_typed() — peek at the top typed value without removing it.
sub peek_typed {
    my ($self) = @_;
    CodingAdventures::VirtualMachine::StackUnderflowError->throw(
        'Cannot peek an empty typed stack'
    ) if @{ $self->{typed_stack} } == 0;
    return $self->{typed_stack}[-1];
}

# ============================================================================
# CONTEXT OPCODE REGISTRATION — Register handlers for domain-specific opcodes
# ============================================================================
#
# register_context_opcode($opcode, $handler) — register a handler sub for
# an opcode that receives (vm, instr, code, context) as arguments.
# These handlers are used by the WASM execution engine to add WASM-specific
# instruction semantics to the generic VM.

sub register_context_opcode {
    my ($self, $opcode, $handler) = @_;
    $self->{_context_handlers}{$opcode} = $handler;
}

# ============================================================================
# EXECUTE WITH CONTEXT — Run code with a domain-specific context object
# ============================================================================
#
# execute_with_context($code, $context) — execute a CodeObject with the given
# context object available to all context opcode handlers.

sub execute_with_context {
    my ($self, $code, $context) = @_;
    $self->{_current_context} = $context;
    $self->{_program} = $code;
    while ( !$self->{halted} ) {
        my $current_code = $self->{_program} || $code;
        my $instrs = $current_code->instructions();
        last if $self->{pc} >= scalar(@$instrs);
        # Call pre-step hook if registered
        if ($self->{_pre_step_hook}) {
            $self->{_pre_step_hook}->($self, $current_code, $context);
        }
        $self->step($current_code);
    }
    $self->{_current_context} = undef;
}

# ============================================================================
# HOOKS AND CONFIGURATION
# ============================================================================

# set_pre_step_hook($coderef) — register a hook called before each step.
sub set_pre_step_hook {
    my ($self, $hook) = @_;
    $self->{_pre_step_hook} = $hook;
}

# set_max_recursion_depth($n) — set maximum call depth.
sub set_max_recursion_depth {
    my ($self, $n) = @_;
    $self->{_max_recursion} = $n;
}

# max_recursion_depth() — get maximum call depth.
sub max_recursion_depth {
    return $_[0]->{_max_recursion};
}

# reset() — public alias for _reset. Preserves registered context handlers.
sub reset {
    my ($self) = @_;
    my $handlers = $self->{_context_handlers} || {};
    $self->_reset();
    $self->{_context_handlers} = $handlers;
}

1;

__END__

=head1 NAME

CodingAdventures::VirtualMachine - Pure-Perl stack-based bytecode virtual machine

=head1 SYNOPSIS

    use CodingAdventures::VirtualMachine;

    my $vm = CodingAdventures::VirtualMachine->new();

    my $code = CodingAdventures::VirtualMachine::CodeObject->new(
        instructions => [
            { opcode => 0x01, operand => 0 },  # LOAD_CONST 10
            { opcode => 0x01, operand => 1 },  # LOAD_CONST 20
            { opcode => 0x20 },                # ADD
            { opcode => 0x60 },                # PRINT
            { opcode => 0xFF },                # HALT
        ],
        constants => [10, 20],
    );

    $vm->execute($code);
    print $vm->output->[0];  # "30"

=head1 DESCRIPTION

A simple stack-based virtual machine implementing the educational bytecode
instruction set used throughout the coding-adventures project.

=head1 OPCODES

    LOAD_CONST  (0x01) -- Push constants[operand] onto stack
    POP         (0x02) -- Discard top of stack
    DUP         (0x03) -- Duplicate top of stack
    STORE_NAME  (0x10) -- Pop and store into named variable
    LOAD_NAME   (0x11) -- Push named variable onto stack
    STORE_LOCAL (0x12) -- Pop and store into local slot
    LOAD_LOCAL  (0x13) -- Push local slot onto stack
    ADD         (0x20) -- Pop b, pop a, push a+b
    SUB         (0x21) -- Pop b, pop a, push a-b
    MUL         (0x22) -- Pop b, pop a, push a*b
    DIV         (0x23) -- Pop b, pop a, push int(a/b)
    CMP_EQ      (0x30) -- Pop b, pop a, push 1 if a==b else 0
    CMP_LT      (0x31) -- Pop b, pop a, push 1 if a<b else 0
    CMP_GT      (0x32) -- Pop b, pop a, push 1 if a>b else 0
    JUMP        (0x40) -- Unconditional jump to operand
    JUMP_IF_FALSE (0x41) -- Pop; jump if falsy
    JUMP_IF_TRUE  (0x42) -- Pop; jump if truthy
    CALL        (0x50) -- Call function named by names[operand]
    RETURN      (0x51) -- Return from function
    PRINT       (0x60) -- Pop and append to output
    HALT        (0xFF) -- Stop execution

=head1 VERSION

Version 0.01

=head1 AUTHOR

coding-adventures

=head1 LICENSE

MIT

=cut
