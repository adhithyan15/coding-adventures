package CodingAdventures::StarlarkCompiler::Opcodes;

use strict;
use warnings;
use v5.28;

use parent 'Exporter';

use CodingAdventures::StarlarkCompiler::Constants;

our @EXPORT = qw(
    all_ops
    op_from_byte
    op_from_symbol
    op_byte
    op_symbol
    op_category
    op_category_from_byte
    binary_op_map
    compare_op_map
    augmented_assign_map
    unary_op_map
    binary_opcode
    compare_opcode
    augmented_assign_opcode
    unary_opcode

    allOps
    opFromByte
    opFromSymbol
    opByte
    opSymbol
    opCategory
    opCategoryFromByte
    binaryOpMap
    compareOpMap
    augmentedAssignMap
    unaryOpMap
    binaryOpcode
    compareOpcode
    augmentedAssignOpcode
    unaryOpcode
);

# 12 categories
my %OpCategory = (
    stack               => { id => 'stack', camel_name => 'stack', nibble => 0x0 },
    variable            => { id => 'variable', camel_name => 'variable', nibble => 0x1 },
    arithmetic          => { id => 'arithmetic', camel_name => 'arithmetic', nibble => 0x2 },
    comparison          => { id => 'comparison', camel_name => 'comparison', nibble => 0x3 },
    control_flow        => { id => 'control_flow', camel_name => 'controlFlow', nibble => 0x4 },
    function            => { id => 'function', camel_name => 'function', nibble => 0x5 },
    collection          => { id => 'collection', camel_name => 'collection', nibble => 0x6 },
    subscript_attribute => { id => 'subscript_attribute', camel_name => 'subscriptAttribute', nibble => 0x7 },
    iteration           => { id => 'iteration', camel_name => 'iteration', nibble => 0x8 },
    module              => { id => 'module', camel_name => 'module', nibble => 0x9 },
    io                  => { id => 'io', camel_name => 'io', nibble => 0xA },
    vm_control          => { id => 'vm_control', camel_name => 'vmControl', nibble => 0xF },
);

# Aliases for category lookups
$OpCategory{controlFlow}        = $OpCategory{control_flow};
$OpCategory{subscriptAttribute} = $OpCategory{subscript_attribute};
$OpCategory{vmControl}          = $OpCategory{vm_control};

my @raw_op_specs = (
    # Stack (0x01..0x06)
    [ 0x01, 'LoadConst', $OpCategory{stack} ],
    [ 0x02, 'Pop',       $OpCategory{stack} ],
    [ 0x03, 'Dup',       $OpCategory{stack} ],
    [ 0x04, 'LoadNone',  $OpCategory{stack} ],
    [ 0x05, 'LoadTrue',  $OpCategory{stack} ],
    [ 0x06, 'LoadFalse', $OpCategory{stack} ],

    # Variable (0x10..0x15)
    [ 0x10, 'StoreName',    $OpCategory{variable} ],
    [ 0x11, 'LoadName',     $OpCategory{variable} ],
    [ 0x12, 'StoreLocal',   $OpCategory{variable} ],
    [ 0x13, 'LoadLocal',    $OpCategory{variable} ],
    [ 0x14, 'StoreClosure', $OpCategory{variable} ],
    [ 0x15, 'LoadClosure',  $OpCategory{variable} ],

    # Arithmetic (0x20..0x2D)
    [ 0x20, 'Add',      $OpCategory{arithmetic} ],
    [ 0x21, 'Sub',      $OpCategory{arithmetic} ],
    [ 0x22, 'Mul',      $OpCategory{arithmetic} ],
    [ 0x23, 'Div',      $OpCategory{arithmetic} ],
    [ 0x24, 'FloorDiv', $OpCategory{arithmetic} ],
    [ 0x25, 'Mod',      $OpCategory{arithmetic} ],
    [ 0x26, 'Power',    $OpCategory{arithmetic} ],
    [ 0x27, 'Negate',   $OpCategory{arithmetic} ],
    [ 0x28, 'BitAnd',   $OpCategory{arithmetic} ],
    [ 0x29, 'BitOr',    $OpCategory{arithmetic} ],
    [ 0x2A, 'BitXor',   $OpCategory{arithmetic} ],
    [ 0x2B, 'BitNot',   $OpCategory{arithmetic} ],
    [ 0x2C, 'LShift',   $OpCategory{arithmetic} ],
    [ 0x2D, 'RShift',   $OpCategory{arithmetic} ],

    # Comparison (0x30..0x38)
    [ 0x30, 'CmpEq',    $OpCategory{comparison} ],
    [ 0x31, 'CmpNe',    $OpCategory{comparison} ],
    [ 0x32, 'CmpLt',    $OpCategory{comparison} ],
    [ 0x33, 'CmpGt',    $OpCategory{comparison} ],
    [ 0x34, 'CmpLe',    $OpCategory{comparison} ],
    [ 0x35, 'CmpGe',    $OpCategory{comparison} ],
    [ 0x36, 'CmpIn',    $OpCategory{comparison} ],
    [ 0x37, 'CmpNotIn', $OpCategory{comparison} ],
    [ 0x38, 'Not',      $OpCategory{comparison} ],

    # Control Flow (0x40..0x44)
    [ 0x40, 'Jump',            $OpCategory{control_flow} ],
    [ 0x41, 'JumpIfFalse',     $OpCategory{control_flow} ],
    [ 0x42, 'JumpIfTrue',      $OpCategory{control_flow} ],
    [ 0x43, 'JumpIfFalseOrPop', $OpCategory{control_flow} ],
    [ 0x44, 'JumpIfTrueOrPop',  $OpCategory{control_flow} ],

    # Function (0x50..0x53)
    [ 0x50, 'MakeFunction',  $OpCategory{function} ],
    [ 0x51, 'CallFunction',  $OpCategory{function} ],
    [ 0x52, 'CallFunctionKw', $OpCategory{function} ],
    [ 0x53, 'Return',        $OpCategory{function} ],

    # Collection (0x60..0x64)
    [ 0x60, 'BuildList',  $OpCategory{collection} ],
    [ 0x61, 'BuildDict',  $OpCategory{collection} ],
    [ 0x62, 'BuildTuple', $OpCategory{collection} ],
    [ 0x63, 'ListAppend', $OpCategory{collection} ],
    [ 0x64, 'DictSet',    $OpCategory{collection} ],

    # Subscript / Attribute (0x70..0x74)
    [ 0x70, 'LoadSubscript',  $OpCategory{subscript_attribute} ],
    [ 0x71, 'StoreSubscript', $OpCategory{subscript_attribute} ],
    [ 0x72, 'LoadAttr',       $OpCategory{subscript_attribute} ],
    [ 0x73, 'StoreAttr',      $OpCategory{subscript_attribute} ],
    [ 0x74, 'LoadSlice',      $OpCategory{subscript_attribute} ],

    # Iteration (0x80..0x82)
    [ 0x80, 'GetIter',        $OpCategory{iteration} ],
    [ 0x81, 'ForIter',        $OpCategory{iteration} ],
    [ 0x82, 'UnpackSequence', $OpCategory{iteration} ],

    # Module (0x90..0x91)
    [ 0x90, 'LoadModule', $OpCategory{module} ],
    [ 0x91, 'ImportFrom', $OpCategory{module} ],

    # IO (0xA0)
    [ 0xA0, 'Print', $OpCategory{io} ],

    # VM Control (0xFF)
    [ 0xFF, 'Halt', $OpCategory{vm_control} ],
);

my %Op;
my @all_ops_list;
my @byte_table = (undef) x 256;
my %symbol_table;

for my $spec (@raw_op_specs) {
    my ($byte_val, $sym, $cat) = @$spec;
    my $op_obj = {
        byte_value => $byte_val,
        symbol     => $sym,
        category   => $cat,
    };
    $Op{$sym} = $op_obj;
    push @all_ops_list, $op_obj;
    $byte_table[$byte_val] = $op_obj;
    $symbol_table{$sym} = $op_obj;
}

# 12 standard binary operators
my %binary_ops = (
    '+'  => $Op{Add},
    '-'  => $Op{Sub},
    '*'  => $Op{Mul},
    '/'  => $Op{Div},
    '//' => $Op{FloorDiv},
    '%'  => $Op{Mod},
    '**' => $Op{Power},
    '&'  => $Op{BitAnd},
    '|'  => $Op{BitOr},
    '^'  => $Op{BitXor},
    '<<' => $Op{LShift},
    '>>' => $Op{RShift},
);

# 8 comparison operators
my %compare_ops = (
    '=='     => $Op{CmpEq},
    '!='     => $Op{CmpNe},
    '<'      => $Op{CmpLt},
    '>'      => $Op{CmpGt},
    '<='     => $Op{CmpLe},
    '>='     => $Op{CmpGe},
    'in'     => $Op{CmpIn},
    'not in' => $Op{CmpNotIn},
);

# 12 augmented assignment operators
my %augmented_assign_ops = (
    '+='  => $Op{Add},
    '-='  => $Op{Sub},
    '*='  => $Op{Mul},
    '/='  => $Op{Div},
    '//=' => $Op{FloorDiv},
    '%='  => $Op{Mod},
    '**=' => $Op{Power},
    '&='  => $Op{BitAnd},
    '|='  => $Op{BitOr},
    '^='  => $Op{BitXor},
    '<<=' => $Op{LShift},
    '>>=' => $Op{RShift},
);

# 2 unary operators
my %unary_ops = (
    '-' => $Op{Negate},
    '~' => $Op{BitNot},
);

sub all_ops {
    return [ @all_ops_list ];
}

sub op_from_byte {
    my ($byte_val) = @_;
    return undef unless defined $byte_val;
    return undef if ref($byte_val);
    return undef unless $byte_val =~ /\A[0-9]+\z/;
    return undef if $byte_val < MIN_BYTE_VALUE || $byte_val > MAX_BYTE_VALUE;
    return $byte_table[$byte_val];
}

sub op_from_symbol {
    my ($sym) = @_;
    return undef unless defined $sym;
    return $symbol_table{$sym};
}

sub op_byte {
    my ($op) = @_;
    return $op ? $op->{byte_value} : undef;
}

sub op_symbol {
    my ($op) = @_;
    return $op ? $op->{symbol} : undef;
}

sub op_category {
    my ($op) = @_;
    return $op ? $op->{category} : undef;
}

sub op_category_from_byte {
    my ($byte_val) = @_;
    my $op = op_from_byte($byte_val);
    return $op ? $op->{category} : undef;
}

sub binary_op_map {
    return { %binary_ops };
}

sub compare_op_map {
    return { %compare_ops };
}

sub augmented_assign_map {
    return { %augmented_assign_ops };
}

sub unary_op_map {
    return { %unary_ops };
}

sub binary_opcode {
    my ($sym) = @_;
    return defined $sym ? $binary_ops{$sym} : undef;
}

sub compare_opcode {
    my ($sym) = @_;
    return defined $sym ? $compare_ops{$sym} : undef;
}

sub augmented_assign_opcode {
    my ($sym) = @_;
    return defined $sym ? $augmented_assign_ops{$sym} : undef;
}

sub unary_opcode {
    my ($sym) = @_;
    return defined $sym ? $unary_ops{$sym} : undef;
}

# CamelCase aliases
*allOps               = \&all_ops;
*opFromByte           = \&op_from_byte;
*opFromSymbol         = \&op_from_symbol;
*opByte               = \&op_byte;
*opSymbol             = \&op_symbol;
*opCategory           = \&op_category;
*opCategoryFromByte   = \&op_category_from_byte;
*binaryOpMap          = \&binary_op_map;
*compareOpMap         = \&compare_op_map;
*augmentedAssignMap   = \&augmented_assign_map;
*unaryOpMap           = \&unary_op_map;
*binaryOpcode         = \&binary_opcode;
*compareOpcode        = \&compare_opcode;
*augmentedAssignOpcode = \&augmented_assign_opcode;
*unaryOpcode          = \&unary_opcode;

1;
