//! AVM2 ActionScript Bytecode (ABC) opcodes.

#![deny(unsafe_code)]

pub const OP_PUSHBYTE: u8 = 0x24;
pub const OP_PUSHSHORT: u8 = 0x25;
pub const OP_PUSHTRUE: u8 = 0x26;
pub const OP_PUSHFALSE: u8 = 0x27;
pub const OP_PUSHNAN: u8 = 0x28;
pub const OP_POP: u8 = 0x29;
pub const OP_DUP: u8 = 0x2A;
pub const OP_SWAP: u8 = 0x2B;
pub const OP_PUSHSTRING: u8 = 0x2C;
pub const OP_PUSHINT: u8 = 0x2D;
pub const OP_PUSHUINT: u8 = 0x2E;
pub const OP_PUSHDOUBLE: u8 = 0x2F;
pub const OP_PUSHNULL: u8 = 0x20;
pub const OP_PUSHUNDEFINED: u8 = 0x21;

pub const OP_GETLOCAL: u8 = 0x62;
pub const OP_SETLOCAL: u8 = 0x63;
pub const OP_GETLOCAL_0: u8 = 0xD0;
pub const OP_GETLOCAL_1: u8 = 0xD1;
pub const OP_GETLOCAL_2: u8 = 0xD2;
pub const OP_GETLOCAL_3: u8 = 0xD3;
pub const OP_SETLOCAL_0: u8 = 0xD4;
pub const OP_SETLOCAL_1: u8 = 0xD5;
pub const OP_SETLOCAL_2: u8 = 0xD6;
pub const OP_SETLOCAL_3: u8 = 0xD7;

pub const OP_ADD: u8 = 0xA0;
pub const OP_SUBTRACT: u8 = 0xA1;
pub const OP_MULTIPLY: u8 = 0xA2;
pub const OP_DIVIDE: u8 = 0xA3;
pub const OP_MODULO: u8 = 0xA4;
pub const OP_NEGATE: u8 = 0x90;
pub const OP_INCREMENT: u8 = 0x91;
pub const OP_DECREMENT: u8 = 0x93;
pub const OP_NOT: u8 = 0x96;

pub const OP_EQUALS: u8 = 0xAB;
pub const OP_STRICTEQUALS: u8 = 0xAC;
pub const OP_LESSTHAN: u8 = 0xAD;
pub const OP_LESSEQUALS: u8 = 0xAE;
pub const OP_GREATERTHAN: u8 = 0xAF;
pub const OP_GREATEREQUALS: u8 = 0xB0;

pub const OP_JUMP: u8 = 0x10;
pub const OP_IFTRUE: u8 = 0x11;
pub const OP_IFFALSE: u8 = 0x12;
pub const OP_IFEQ: u8 = 0x13;
pub const OP_IFNE: u8 = 0x14;
pub const OP_IFLT: u8 = 0x15;
pub const OP_IFLE: u8 = 0x16;
pub const OP_IFGT: u8 = 0x17;
pub const OP_IFGE: u8 = 0x18;

pub const OP_FINDPROPSTRICT: u8 = 0x5D;
pub const OP_GETPROPERTY: u8 = 0x66;
pub const OP_SETPROPERTY: u8 = 0x61;
pub const OP_INITPROPERTY: u8 = 0x68;
pub const OP_CALLPROPERTY: u8 = 0x46;
pub const OP_CALLPROPVOID: u8 = 0x4F;
pub const OP_CONSTRUCTPROP: u8 = 0x4A;
pub const OP_RETURNVALUE: u8 = 0x48;
pub const OP_RETURNVOID: u8 = 0x47;
