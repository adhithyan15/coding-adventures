/// Pure Starlark bytecode opcode definitions, category enumeration,
/// and bidirectional operator lookup tables.
///
/// Designed as an inert, deterministic opcode provider without dependencies
/// on a parser, runtime, or host platform.
library;

import 'constants.dart';

/// Semantic functional category for Starlark bytecode instructions.
///
/// Each category corresponds to an architectural family of bytecode operations,
/// encoded in the high nibble of each opcode's byte value.
enum OpCategory {
  /// Stack management and constant loading operations (0x0X).
  stack('stack', 'stack', 0x0),

  /// Variable, local scope, and closure resolution operations (0x1X).
  variable('variable', 'variable', 0x1),

  /// Arithmetic and bitwise computation operations (0x2X).
  arithmetic('arithmetic', 'arithmetic', 0x2),

  /// Relational comparison and membership operations (0x3X).
  comparison('comparison', 'comparison', 0x3),

  /// Unconditional and conditional branching instructions (0x4X).
  controlFlow('control_flow', 'controlFlow', 0x4),

  /// Function closure creation, invocation, and return operations (0x5X).
  function_('function', 'function', 0x5),

  /// List, dictionary, and tuple construction operations (0x6X).
  collection('collection', 'collection', 0x6),

  /// Subscript indexing, slice extraction, and attribute operations (0x7X).
  subscriptAttribute('subscript_attribute', 'subscriptAttribute', 0x7),

  /// Sequence iteration and unpacking operations (0x8X).
  iteration('iteration', 'iteration', 0x8),

  /// Module loading and imported symbol binding operations (0x9X).
  module('module', 'module', 0x9),

  /// Standard I/O output operations (0xAX).
  io('io', 'io', 0xA),

  /// Virtual machine execution control and termination operations (0xFX).
  vmControl('vm_control', 'vmControl', 0xF);

  const OpCategory(this.id, this.camelName, this.nibble);

  /// Snake-case identifier matching specification fixtures (e.g., `'control_flow'`).
  final String id;

  /// Camel-case identifier matching TypeScript / JS conventions (e.g., `'controlFlow'`).
  final String camelName;

  /// High nibble associated with this category family.
  final int nibble;

  @override
  String toString() => id;
}

/// The 61 canonical Starlark bytecode opcodes defined in the Starlark bytecode specification.
enum Op {
  // Stack (0x01..0x06)
  /// Push constant from pool onto evaluation stack.
  loadConst(0x01, 'LoadConst', OpCategory.stack),

  /// Discard top-of-stack item.
  pop(0x02, 'Pop', OpCategory.stack),

  /// Duplicate top-of-stack item.
  dup(0x03, 'Dup', OpCategory.stack),

  /// Push singleton `None` onto evaluation stack.
  loadNone(0x04, 'LoadNone', OpCategory.stack),

  /// Push boolean `True` onto evaluation stack.
  loadTrue(0x05, 'LoadTrue', OpCategory.stack),

  /// Push boolean `False` onto evaluation stack.
  loadFalse(0x06, 'LoadFalse', OpCategory.stack),

  // Variable (0x10..0x15)
  /// Store top-of-stack value into named global variable.
  storeName(0x10, 'StoreName', OpCategory.variable),

  /// Load value of named global variable onto stack.
  loadName(0x11, 'LoadName', OpCategory.variable),

  /// Store top-of-stack value into indexed local slot.
  storeLocal(0x12, 'StoreLocal', OpCategory.variable),

  /// Load value of indexed local slot onto stack.
  loadLocal(0x13, 'LoadLocal', OpCategory.variable),

  /// Store top-of-stack value into closure free-variable slot.
  storeClosure(0x14, 'StoreClosure', OpCategory.variable),

  /// Load value from closure free-variable slot onto stack.
  loadClosure(0x15, 'LoadClosure', OpCategory.variable),

  // Arithmetic (0x20..0x2D)
  /// Binary addition (`+`).
  add(0x20, 'Add', OpCategory.arithmetic),

  /// Binary subtraction (`-`).
  sub(0x21, 'Sub', OpCategory.arithmetic),

  /// Binary multiplication (`*`).
  mul(0x22, 'Mul', OpCategory.arithmetic),

  /// Binary real division (`/`).
  div(0x23, 'Div', OpCategory.arithmetic),

  /// Binary floor division (`//`).
  floorDiv(0x24, 'FloorDiv', OpCategory.arithmetic),

  /// Binary modulo remainder (`%`).
  mod(0x25, 'Mod', OpCategory.arithmetic),

  /// Binary exponentiation (`**`).
  power(0x26, 'Power', OpCategory.arithmetic),

  /// Unary arithmetic negation (`-`).
  negate(0x27, 'Negate', OpCategory.arithmetic),

  /// Bitwise AND (`&`).
  bitAnd(0x28, 'BitAnd', OpCategory.arithmetic),

  /// Bitwise OR (`|`).
  bitOr(0x29, 'BitOr', OpCategory.arithmetic),

  /// Bitwise XOR (`^`).
  bitXor(0x2A, 'BitXor', OpCategory.arithmetic),

  /// Bitwise NOT invert (`~`).
  bitNot(0x2B, 'BitNot', OpCategory.arithmetic),

  /// Bitwise left shift (`<<`).
  lShift(0x2C, 'LShift', OpCategory.arithmetic),

  /// Bitwise right shift (`>>`).
  rShift(0x2D, 'RShift', OpCategory.arithmetic),

  // Comparison (0x30..0x38)
  /// Equality comparison (`==`).
  cmpEq(0x30, 'CmpEq', OpCategory.comparison),

  /// Inequality comparison (`!=`).
  cmpNe(0x31, 'CmpNe', OpCategory.comparison),

  /// Strictly less-than comparison (`<`).
  cmpLt(0x32, 'CmpLt', OpCategory.comparison),

  /// Strictly greater-than comparison (`>`).
  cmpGt(0x33, 'CmpGt', OpCategory.comparison),

  /// Less-than-or-equal comparison (`<=`).
  cmpLe(0x34, 'CmpLe', OpCategory.comparison),

  /// Greater-than-or-equal comparison (`>=`).
  cmpGe(0x35, 'CmpGe', OpCategory.comparison),

  /// Collection containment membership test (`in`).
  cmpIn(0x36, 'CmpIn', OpCategory.comparison),

  /// Negative collection containment membership test (`not in`).
  cmpNotIn(0x37, 'CmpNotIn', OpCategory.comparison),

  /// Boolean logical negation (`not`).
  not_(0x38, 'Not', OpCategory.comparison),

  // Control Flow (0x40..0x44)
  /// Unconditional relative or absolute jump.
  jump(0x40, 'Jump', OpCategory.controlFlow),

  /// Jump if top-of-stack is falsey, popping condition.
  jumpIfFalse(0x41, 'JumpIfFalse', OpCategory.controlFlow),

  /// Jump if top-of-stack is truthy, popping condition.
  jumpIfTrue(0x42, 'JumpIfTrue', OpCategory.controlFlow),

  /// Jump if top-of-stack is falsey, preserving value on branch.
  jumpIfFalseOrPop(0x43, 'JumpIfFalseOrPop', OpCategory.controlFlow),

  /// Jump if top-of-stack is truthy, preserving value on branch.
  jumpIfTrueOrPop(0x44, 'JumpIfTrueOrPop', OpCategory.controlFlow),

  // Function (0x50..0x53)
  /// Create executable function object from code descriptor.
  makeFunction(0x50, 'MakeFunction', OpCategory.function_),

  /// Call function with positional arguments.
  callFunction(0x51, 'CallFunction', OpCategory.function_),

  /// Call function with keyword and positional arguments.
  callFunctionKw(0x52, 'CallFunctionKw', OpCategory.function_),

  /// Return top-of-stack value from current frame.
  return_(0x53, 'Return', OpCategory.function_),

  // Collection (0x60..0x64)
  /// Construct list of N items popped from stack.
  buildList(0x60, 'BuildList', OpCategory.collection),

  /// Construct dictionary of N key-value pairs popped from stack.
  buildDict(0x61, 'BuildDict', OpCategory.collection),

  /// Construct immutable tuple of N items popped from stack.
  buildTuple(0x62, 'BuildTuple', OpCategory.collection),

  /// Append item to target list container.
  listAppend(0x63, 'ListAppend', OpCategory.collection),

  /// Set key-value pair in target dictionary container.
  dictSet(0x64, 'DictSet', OpCategory.collection),

  // Subscript / Attribute (0x70..0x74)
  /// Load item from container by subscript key/index.
  loadSubscript(0x70, 'LoadSubscript', OpCategory.subscriptAttribute),

  /// Store item into container at subscript key/index.
  storeSubscript(0x71, 'StoreSubscript', OpCategory.subscriptAttribute),

  /// Load attribute from object by symbol name.
  loadAttr(0x72, 'LoadAttr', OpCategory.subscriptAttribute),

  /// Store attribute onto object by symbol name.
  storeAttr(0x73, 'StoreAttr', OpCategory.subscriptAttribute),

  /// Extract slice [start:stop:step] from sequence.
  loadSlice(0x74, 'LoadSlice', OpCategory.subscriptAttribute),

  // Iteration (0x80..0x82)
  /// Construct iterator object from top-of-stack iterable.
  getIter(0x80, 'GetIter', OpCategory.iteration),

  /// Advance iterator and branch if sequence is exhausted.
  forIter(0x81, 'ForIter', OpCategory.iteration),

  /// Unpack sequence elements into N stack entries.
  unpackSequence(0x82, 'UnpackSequence', OpCategory.iteration),

  // Module (0x90..0x91)
  /// Resolve and import external Starlark module.
  loadModule(0x90, 'LoadModule', OpCategory.module),

  /// Extract symbol from imported module into current frame.
  importFrom(0x91, 'ImportFrom', OpCategory.module),

  // IO (0xA0)
  /// Emit values to standard output device.
  print_(0xA0, 'Print', OpCategory.io),

  // VM Control (0xFF)
  /// Terminate virtual machine evaluation cycle.
  halt(0xFF, 'Halt', OpCategory.vmControl);

  const Op(this.byteValue, this.symbol, this.category);

  /// 8-bit unsigned bytecode representation.
  final int byteValue;

  /// Canonical PascalCase identifier name.
  final String symbol;

  /// Functional opcode category.
  final OpCategory category;

  @override
  String toString() => symbol;
}

/// Precomputed 256-element direct lookup table from byte values to [Op].
final List<Op?> _byteTable = () {
  final table = List<Op?>.filled(expectedDecodeCount, null);
  for (final op in Op.values) {
    table[op.byteValue] = op;
  }
  return List<Op?>.unmodifiable(table);
}();

/// Fast lookup table from canonical symbol name to [Op].
final Map<String, Op> _symbolTable = {
  for (final op in Op.values) op.symbol: op,
};

/// 12 standard binary operators mapped to their corresponding Starlark arithmetic opcodes.
const Map<String, Op> _binaryOps = {
  '+': Op.add,
  '-': Op.sub,
  '*': Op.mul,
  '/': Op.div,
  '//': Op.floorDiv,
  '%': Op.mod,
  '**': Op.power,
  '&': Op.bitAnd,
  '|': Op.bitOr,
  '^': Op.bitXor,
  '<<': Op.lShift,
  '>>': Op.rShift,
};

/// 8 relational and membership comparison operators.
const Map<String, Op> _compareOps = {
  '==': Op.cmpEq,
  '!=': Op.cmpNe,
  '<': Op.cmpLt,
  '>': Op.cmpGt,
  '<=': Op.cmpLe,
  '>=': Op.cmpGe,
  'in': Op.cmpIn,
  'not in': Op.cmpNotIn,
};

/// 12 augmented in-place assignment operators.
const Map<String, Op> _augmentedAssignOps = {
  '+=': Op.add,
  '-=': Op.sub,
  '*=': Op.mul,
  '/=': Op.div,
  '//=': Op.floorDiv,
  '%=': Op.mod,
  '**=': Op.power,
  '&=': Op.bitAnd,
  '|=': Op.bitOr,
  '^=': Op.bitXor,
  '<<=': Op.lShift,
  '>>=': Op.rShift,
};

/// 2 unary operators mapped to arithmetic opcodes.
const Map<String, Op> _unaryOps = {
  '-': Op.negate,
  '~': Op.bitNot,
};

/// Returns a fresh defensive copy of all 61 Starlark opcodes in canonical order.
///
/// Modifying the returned list does not affect internal opcode provider tables.
List<Op> allOps() => List<Op>.of(Op.values);

/// Decodes an 8-bit byte value into its corresponding [Op].
///
/// Returns `null` if [value] is outside 0..255 or falls into an unassigned byte hole.
Op? opFromByte(int value) {
  if (value < minByteValue || value > maxByteValue) {
    return null;
  }
  return _byteTable[value];
}

/// Resolves an [Op] from its canonical PascalCase symbol string.
///
/// Returns `null` if the symbol is unrecognized.
Op? opFromSymbol(String symbol) => _symbolTable[symbol];

/// Returns the 8-bit unsigned numeric value of [op].
int opByte(Op op) => op.byteValue;

/// Returns the canonical PascalCase symbol string of [op].
String opSymbol(Op op) => op.symbol;

/// Returns the functional [OpCategory] of [op].
OpCategory opCategory(Op op) => op.category;

/// Resolves the [OpCategory] associated with the given 8-bit [value].
///
/// Returns `null` if [value] is out of range or unassigned.
OpCategory? opCategoryFromByte(int value) => opFromByte(value)?.category;

/// Returns a fresh, mutable mapping of all 12 binary arithmetic operator symbols to opcodes.
///
/// Complies with the fresh-map ownership contract: mutations to this returned
/// map never alter subsequent lookups or internal state.
Map<String, Op> binaryOpMap() => Map<String, Op>.of(_binaryOps);

/// Returns a fresh, mutable mapping of all 8 comparison operator symbols to opcodes.
///
/// Complies with the fresh-map ownership contract: mutations to this returned
/// map never alter subsequent lookups or internal state.
Map<String, Op> compareOpMap() => Map<String, Op>.of(_compareOps);

/// Returns a fresh, mutable mapping of all 12 augmented assignment operators to opcodes.
///
/// Complies with the fresh-map ownership contract: mutations to this returned
/// map never alter subsequent lookups or internal state.
Map<String, Op> augmentedAssignMap() => Map<String, Op>.of(_augmentedAssignOps);

/// Returns a fresh, mutable mapping of all 2 unary operators to opcodes.
///
/// Complies with the fresh-map ownership contract: mutations to this returned
/// map never alter subsequent lookups or internal state.
Map<String, Op> unaryOpMap() => Map<String, Op>.of(_unaryOps);

/// Looks up the opcode for a binary arithmetic operator symbol.
///
/// Returns `null` if [operator] is unmapped or unknown.
Op? binaryOpcode(String operator) => _binaryOps[operator];

/// Looks up the opcode for a relational comparison or membership operator symbol.
///
/// Returns `null` if [operator] is unmapped or unknown.
Op? compareOpcode(String operator) => _compareOps[operator];

/// Looks up the opcode for an augmented assignment operator symbol.
///
/// Returns `null` if [operator] is unmapped or unknown.
Op? augmentedAssignOpcode(String operator) => _augmentedAssignOps[operator];

/// Looks up the opcode for a unary operator symbol.
///
/// Returns `null` if [operator] is unmapped or unknown.
Op? unaryOpcode(String operator) => _unaryOps[operator];
