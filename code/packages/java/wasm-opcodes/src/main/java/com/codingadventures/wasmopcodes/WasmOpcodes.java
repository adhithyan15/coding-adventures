package com.codingadventures.wasmopcodes;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;

public final class WasmOpcodes {
    public static final String VERSION = "0.1.0";

    private static final List<OpcodeInfo> RAW_OPCODES = List.of(
            new OpcodeInfo(0x00, "unreachable", "control", List.of(), 0, 0),
            new OpcodeInfo(0x01, "nop", "control", List.of(), 0, 0),
            new OpcodeInfo(0x02, "block", "control", List.of("blocktype"), 0, 0),
            new OpcodeInfo(0x03, "loop", "control", List.of("blocktype"), 0, 0),
            new OpcodeInfo(0x04, "if", "control", List.of("blocktype"), 1, 0),
            new OpcodeInfo(0x05, "else", "control", List.of(), 0, 0),
            new OpcodeInfo(0x0B, "end", "control", List.of(), 0, 0),
            new OpcodeInfo(0x0C, "br", "control", List.of("labelidx"), 0, 0),
            new OpcodeInfo(0x0D, "br_if", "control", List.of("labelidx"), 1, 0),
            new OpcodeInfo(0x0E, "br_table", "control", List.of("vec_labelidx"), 1, 0),
            new OpcodeInfo(0x0F, "return", "control", List.of(), 0, 0),
            new OpcodeInfo(0x10, "call", "control", List.of("funcidx"), 0, 0),
            new OpcodeInfo(0x11, "call_indirect", "control", List.of("typeidx", "tableidx"), 1, 0),
            new OpcodeInfo(0x1A, "drop", "parametric", List.of(), 1, 0),
            new OpcodeInfo(0x1B, "select", "parametric", List.of(), 3, 1),
            new OpcodeInfo(0x20, "local.get", "variable", List.of("localidx"), 0, 1),
            new OpcodeInfo(0x21, "local.set", "variable", List.of("localidx"), 1, 0),
            new OpcodeInfo(0x22, "local.tee", "variable", List.of("localidx"), 1, 1),
            new OpcodeInfo(0x23, "global.get", "variable", List.of("globalidx"), 0, 1),
            new OpcodeInfo(0x24, "global.set", "variable", List.of("globalidx"), 1, 0),
            new OpcodeInfo(0x28, "i32.load", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x29, "i64.load", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2A, "f32.load", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2B, "f64.load", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2C, "i32.load8_s", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2D, "i32.load8_u", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2E, "i32.load16_s", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x2F, "i32.load16_u", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x30, "i64.load8_s", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x31, "i64.load8_u", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x32, "i64.load16_s", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x33, "i64.load16_u", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x34, "i64.load32_s", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x35, "i64.load32_u", "memory", List.of("memarg"), 1, 1),
            new OpcodeInfo(0x36, "i32.store", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x37, "i64.store", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x38, "f32.store", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x39, "f64.store", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3A, "i32.store8", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3B, "i32.store16", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3C, "i64.store8", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3D, "i64.store16", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3E, "i64.store32", "memory", List.of("memarg"), 2, 0),
            new OpcodeInfo(0x3F, "memory.size", "memory", List.of("memidx"), 0, 1),
            new OpcodeInfo(0x40, "memory.grow", "memory", List.of("memidx"), 1, 1),
            new OpcodeInfo(0x41, "i32.const", "numeric_i32", List.of("i32"), 0, 1),
            new OpcodeInfo(0x45, "i32.eqz", "numeric_i32", List.of(), 1, 1),
            new OpcodeInfo(0x46, "i32.eq", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x47, "i32.ne", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x48, "i32.lt_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x49, "i32.lt_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4A, "i32.gt_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4B, "i32.gt_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4C, "i32.le_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4D, "i32.le_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4E, "i32.ge_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x4F, "i32.ge_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x67, "i32.clz", "numeric_i32", List.of(), 1, 1),
            new OpcodeInfo(0x68, "i32.ctz", "numeric_i32", List.of(), 1, 1),
            new OpcodeInfo(0x69, "i32.popcnt", "numeric_i32", List.of(), 1, 1),
            new OpcodeInfo(0x6A, "i32.add", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x6B, "i32.sub", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x6C, "i32.mul", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x6D, "i32.div_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x6E, "i32.div_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x6F, "i32.rem_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x70, "i32.rem_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x71, "i32.and", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x72, "i32.or", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x73, "i32.xor", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x74, "i32.shl", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x75, "i32.shr_s", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x76, "i32.shr_u", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x77, "i32.rotl", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x78, "i32.rotr", "numeric_i32", List.of(), 2, 1),
            new OpcodeInfo(0x42, "i64.const", "numeric_i64", List.of("i64"), 0, 1),
            new OpcodeInfo(0x50, "i64.eqz", "numeric_i64", List.of(), 1, 1),
            new OpcodeInfo(0x51, "i64.eq", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x52, "i64.ne", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x53, "i64.lt_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x54, "i64.lt_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x55, "i64.gt_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x56, "i64.gt_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x57, "i64.le_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x58, "i64.le_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x59, "i64.ge_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x5A, "i64.ge_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x79, "i64.clz", "numeric_i64", List.of(), 1, 1),
            new OpcodeInfo(0x7A, "i64.ctz", "numeric_i64", List.of(), 1, 1),
            new OpcodeInfo(0x7B, "i64.popcnt", "numeric_i64", List.of(), 1, 1),
            new OpcodeInfo(0x7C, "i64.add", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x7D, "i64.sub", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x7E, "i64.mul", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x7F, "i64.div_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x80, "i64.div_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x81, "i64.rem_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x82, "i64.rem_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x83, "i64.and", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x84, "i64.or", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x85, "i64.xor", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x86, "i64.shl", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x87, "i64.shr_s", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x88, "i64.shr_u", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x89, "i64.rotl", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x8A, "i64.rotr", "numeric_i64", List.of(), 2, 1),
            new OpcodeInfo(0x43, "f32.const", "numeric_f32", List.of("f32"), 0, 1),
            new OpcodeInfo(0x5B, "f32.eq", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x5C, "f32.ne", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x5D, "f32.lt", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x5E, "f32.gt", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x5F, "f32.le", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x60, "f32.ge", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x8B, "f32.abs", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x8C, "f32.neg", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x8D, "f32.ceil", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x8E, "f32.floor", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x8F, "f32.trunc", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x90, "f32.nearest", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x91, "f32.sqrt", "numeric_f32", List.of(), 1, 1),
            new OpcodeInfo(0x92, "f32.add", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x93, "f32.sub", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x94, "f32.mul", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x95, "f32.div", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x96, "f32.min", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x97, "f32.max", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x98, "f32.copysign", "numeric_f32", List.of(), 2, 1),
            new OpcodeInfo(0x44, "f64.const", "numeric_f64", List.of("f64"), 0, 1),
            new OpcodeInfo(0x61, "f64.eq", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x62, "f64.ne", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x63, "f64.lt", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x64, "f64.gt", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x65, "f64.le", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x66, "f64.ge", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0x99, "f64.abs", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9A, "f64.neg", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9B, "f64.ceil", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9C, "f64.floor", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9D, "f64.trunc", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9E, "f64.nearest", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0x9F, "f64.sqrt", "numeric_f64", List.of(), 1, 1),
            new OpcodeInfo(0xA0, "f64.add", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA1, "f64.sub", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA2, "f64.mul", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA3, "f64.div", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA4, "f64.min", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA5, "f64.max", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA6, "f64.copysign", "numeric_f64", List.of(), 2, 1),
            new OpcodeInfo(0xA7, "i32.wrap_i64", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xA8, "i32.trunc_f32_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xA9, "i32.trunc_f32_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAA, "i32.trunc_f64_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAB, "i32.trunc_f64_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAC, "i64.extend_i32_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAD, "i64.extend_i32_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAE, "i64.trunc_f32_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xAF, "i64.trunc_f32_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB0, "i64.trunc_f64_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB1, "i64.trunc_f64_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB2, "f32.convert_i32_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB3, "f32.convert_i32_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB4, "f32.convert_i64_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB5, "f32.convert_i64_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB6, "f32.demote_f64", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB7, "f64.convert_i32_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB8, "f64.convert_i32_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xB9, "f64.convert_i64_s", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBA, "f64.convert_i64_u", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBB, "f64.promote_f32", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBC, "i32.reinterpret_f32", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBD, "i64.reinterpret_f64", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBE, "f32.reinterpret_i32", "conversion", List.of(), 1, 1),
            new OpcodeInfo(0xBF, "f64.reinterpret_i64", "conversion", List.of(), 1, 1)
    );

    public static final Map<Integer, OpcodeInfo> OPCODES;
    public static final Map<String, OpcodeInfo> OPCODES_BY_NAME;

    static {
        Map<Integer, OpcodeInfo> byOpcode = new LinkedHashMap<>();
        Map<String, OpcodeInfo> byName = new LinkedHashMap<>();
        for (OpcodeInfo info : RAW_OPCODES) {
            byOpcode.put(info.opcode(), info);
            byName.put(info.name(), info);
        }
        OPCODES = Map.copyOf(byOpcode);
        OPCODES_BY_NAME = Map.copyOf(byName);
    }

    private WasmOpcodes() {}

    public record OpcodeInfo(
            int opcode,
            String name,
            String category,
            List<String> immediates,
            int stackPop,
            int stackPush
    ) {
        public OpcodeInfo {
            Objects.requireNonNull(name, "name");
            Objects.requireNonNull(category, "category");
            immediates = List.copyOf(immediates);
        }
    }

    public static OpcodeInfo getOpcode(int opcode) {
        return OPCODES.get(opcode);
    }

    public static OpcodeInfo getOpcodeByName(String name) {
        return OPCODES_BY_NAME.get(name);
    }
}
