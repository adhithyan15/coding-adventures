# frozen_string_literal: true

# parser.rb — Parse a raw .wasm binary into a structured WasmModule
#
# ─────────────────────────────────────────────────────────────────────────────
# WHAT IS A .wasm FILE?
# ─────────────────────────────────────────────────────────────────────────────
#
# A .wasm file is the compiled binary format of a WebAssembly (WASM) module.
# Compilers like Emscripten, wasm-pack, or clang produce these files.
# The format is compact, fast to validate, and safe (strongly typed).
#
# This parser reads those bytes and builds an in-memory WasmModule object —
# structured data you can inspect, transform, or generate code from.
#
# ─────────────────────────────────────────────────────────────────────────────
# BINARY LAYOUT — ASCII DIAGRAM
# ─────────────────────────────────────────────────────────────────────────────
#
#  Byte offset  Content
#  ──────────── ─────────────────────────────────────────────────────────────
#  0x00         Magic: 0x00 0x61 0x73 0x6D  ("\0asm")
#  0x04         Version: 0x01 0x00 0x00 0x00  (= 1, little-endian)
#  0x08         [Section]*  (zero or more sections follow)
#
#  Each section has the format:
#
#    ┌──────────────────────────────────────────────────────────┐
#    │  id      : 1 byte       — section type (0–11)            │
#    │  size    : u32 ULEB128  — byte count of payload          │
#    │  payload : size bytes   — section-specific content       │
#    └──────────────────────────────────────────────────────────┘
#
#  Section IDs:
#    0  = Custom    (can appear anywhere, zero or more times)
#    1  = Type      (function signatures)
#    2  = Import    (host imports)
#    3  = Function  (type indices for local functions)
#    4  = Table     (indirect call tables)
#    5  = Memory    (linear memory declarations)
#    6  = Global    (global variables)
#    7  = Export    (host-visible exports)
#    8  = Start     (entry-point function index)
#    9  = Element   (table initializers)
#    10 = Code      (function bodies / bytecode)
#    11 = Data      (memory initializers)
#
#  Numbered sections (1–11) must appear in ascending order.
#  Custom sections (0) may appear between any two sections.
#
# ─────────────────────────────────────────────────────────────────────────────
# DESIGN PHILOSOPHY
# ─────────────────────────────────────────────────────────────────────────────
#
# This parser uses a cursor-based approach: a single integer @pos tracks where
# we are in the byte array. Every "read" method advances @pos by the number of
# bytes consumed. If something is malformed, WasmParseError is raised with the
# exact byte offset so the caller knows where the problem is.
#
# Analogy: reading a book page by page. We know exactly which page we're on,
# and if a page is missing, we can report which page number was expected.
#
# ─────────────────────────────────────────────────────────────────────────────

module CodingAdventures
  module WasmModuleParser
    # ─────────────────────────────────────────────────────────────────────────
    # Constants
    # ─────────────────────────────────────────────────────────────────────────

    # The magic four-byte prefix of every valid .wasm file.
    #
    # "\0asm" — 0x00 as the first byte makes tools detect it as binary
    # (not text), preventing accidental corruption by text editors.
    WASM_MAGIC = [0x00, 0x61, 0x73, 0x6D].freeze

    # Version 1: all WASM 1.0 modules use [0x01, 0x00, 0x00, 0x00] (little-endian uint32 = 1)
    WASM_VERSION = [0x01, 0x00, 0x00, 0x00].freeze

    # Section IDs — each number corresponds to a chapter of the module.
    SECTION_CUSTOM   = 0
    SECTION_TYPE     = 1
    SECTION_IMPORT   = 2
    SECTION_FUNCTION = 3
    SECTION_TABLE    = 4
    SECTION_MEMORY   = 5
    SECTION_GLOBAL   = 6
    SECTION_EXPORT   = 7
    SECTION_START    = 8
    SECTION_ELEMENT  = 9
    SECTION_CODE     = 10
    SECTION_DATA     = 11

    # Function type prefix byte — starts every entry in the type section.
    FUNC_TYPE_PREFIX = 0x60

    # End opcode — terminates constant init expressions.
    END_OPCODE = 0x0B

    # ─────────────────────────────────────────────────────────────────────────
    # WasmParseError
    # ─────────────────────────────────────────────────────────────────────────

    # WasmParseError — raised when binary data is malformed.
    #
    # Carries the byte offset where the problem was detected, in addition to
    # the standard error message. This lets callers point users at the exact
    # problematic byte in a hex dump or disassembly.
    #
    # Example:
    #   begin
    #     parser.parse(bad_data)
    #   rescue WasmParseError => e
    #     puts "Parse failed at offset 0x#{e.offset.to_s(16)}: #{e.message}"
    #   end
    class WasmParseError < StandardError
      # The byte offset in the input where the error was detected.
      attr_reader :offset

      # @param message [String] human-readable description of what went wrong
      # @param offset  [Integer] byte offset in the input
      def initialize(message, offset)
        super(message)
        @offset = offset
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # Parser
    # ─────────────────────────────────────────────────────────────────────────

    # Parser — the main entry point for parsing .wasm binary data.
    #
    # Usage:
    #   parser = CodingAdventures::WasmModuleParser::Parser.new
    #   module  = parser.parse(wasm_bytes)   # raises WasmParseError on bad data
    #
    # The parser is stateless between calls — you can reuse the same instance
    # to parse many different modules. All cursor state lives in BinaryReader.
    #
    # @param data [String (binary), Array<Integer>] the raw .wasm bytes
    # @return [WasmTypes::WasmModule] the structured module
    # @raise  [WasmParseError] if the binary is malformed
    class Parser
      include CodingAdventures::WasmTypes

      # parse — decode a .wasm binary into a WasmModule.
      #
      # Accepts either a binary String (encoding "ASCII-8BIT") or an Array of
      # integers (0–255). Both are common in Ruby code that deals with binary.
      #
      # @param data [String, Array<Integer>] raw .wasm bytes
      # @return [WasmTypes::WasmModule]
      def parse(data)
        bytes = data.is_a?(Array) ? data : data.bytes
        reader = BinaryReader.new(bytes)
        reader.parse_module
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # BinaryReader — internal cursor-based parser
    # ─────────────────────────────────────────────────────────────────────────

    # BinaryReader — a stateful cursor over an array of bytes.
    #
    # This is an internal helper class. Every read method advances @pos forward
    # by the number of bytes consumed. Think of it as a tape head reading a tape.
    #
    # ┌──────────────────────────────────────────────────────────┐
    # │  data:  [00][61][73][6D][01][00][00][00][01][05][...] .. │
    # │          ^                                               │
    # │          @pos=0 (start)                                  │
    # └──────────────────────────────────────────────────────────┘
    #
    # After reading the 8-byte header:
    #
    # ┌──────────────────────────────────────────────────────────┐
    # │  data:  [00][61][73][6D][01][00][00][00][01][05][...] .. │
    # │                                          ^               │
    # │                                          @pos=8          │
    # └──────────────────────────────────────────────────────────┘
    class BinaryReader
      include CodingAdventures::WasmTypes
      include CodingAdventures::WasmLeb128

      def initialize(bytes)
        @data = bytes
        @pos  = 0
      end

      # ── Primitive reads ──────────────────────────────────────────────────

      # read_byte — read a single byte, advance cursor.
      #
      # @raise WasmParseError if at end of data
      def read_byte
        raise WasmParseError.new(
          "Unexpected end of data: expected 1 byte at offset #{@pos}",
          @pos
        ) if @pos >= @data.length

        byte = @data[@pos]
        @pos += 1
        byte
      end

      # read_bytes — read exactly n bytes, return as Array<Integer>.
      #
      # @raise WasmParseError if fewer than n bytes remain
      def read_bytes(n)
        raise WasmParseError.new(
          "Unexpected end of data: expected #{n} bytes at offset #{@pos}, " \
          "but only #{@data.length - @pos} remain",
          @pos
        ) if @pos + n > @data.length

        slice = @data[@pos, n]
        @pos += n
        slice
      end

      # read_u32 — read a ULEB128-encoded unsigned integer.
      #
      # ULEB128 (Unsigned LEB128) is variable-length:
      #   - Each byte contributes 7 bits.
      #   - Bit 7 (0x80) signals "more bytes follow."
      #   - Small numbers (< 128) fit in a single byte.
      #
      # This is how WASM encodes almost every integer: section sizes, counts,
      # indices, limits — all as ULEB128 so small values stay small.
      #
      # Example: 300 = 0b1_0010_1100
      #   Byte 0: 0b1010_1100 = 0xAC (continuation set)
      #   Byte 1: 0b0000_0010 = 0x02 (last byte)
      #
      # @raise WasmParseError on malformed encoding
      def read_u32
        offset = @pos
        value, consumed = CodingAdventures::WasmLeb128.decode_unsigned(@data, @pos)
        @pos += consumed
        value
      rescue CodingAdventures::WasmLeb128::LEB128Error => e
        raise WasmParseError.new("Invalid LEB128 at offset #{offset}: #{e.message}", offset)
      end

      # read_string — read a WASM name string: length:u32leb + UTF-8 bytes.
      #
      # WASM strings are always UTF-8. Names (import names, export names,
      # custom section names) use this encoding.
      #
      # Example — encoding "env" (3 bytes):
      #   [0x03, 0x65, 0x6E, 0x76]
      #    ^^^^ length   e    n    v
      def read_string
        length = read_u32
        bytes  = read_bytes(length)
        bytes.pack("C*").force_encoding("UTF-8")
      end

      # at_end? — true if we've consumed all bytes.
      def at_end?
        @pos >= @data.length
      end

      # pos — current cursor position.
      attr_reader :pos

      # ── Structured reads ────────────────────────────────────────────────

      # read_limits — read a Limits structure.
      #
      # Limits appear in memory and table types.
      #
      # Binary encoding:
      #   flags:u8 = 0x00 → min:u32leb only (no maximum)
      #   flags:u8 = 0x01 → min:u32leb, max:u32leb
      #
      # Bit 0 of flags signals whether a maximum is present:
      #
      #   ┌───────┬────────────────────────────┐
      #   │ flags │ Meaning                    │
      #   ├───────┼────────────────────────────┤
      #   │  0x00 │ min only (unbounded growth) │
      #   │  0x01 │ min + max (bounded growth)  │
      #   └───────┴────────────────────────────┘
      def read_limits
        flags_offset = @pos
        flags = read_byte
        min   = read_u32
        max   = nil
        if flags & 1 == 1
          max = read_u32
        elsif flags != 0
          raise WasmParseError.new(
            "Unknown limits flags byte 0x#{flags.to_s(16)} at offset #{flags_offset}",
            flags_offset
          )
        end
        Limits.new(min, max)
      end

      # read_global_type — read a GlobalType structure.
      #
      # GlobalType = value_type:u8 + mutability:u8
      #
      # Examples:
      #   [0x7F, 0x00] → immutable i32 global
      #   [0x7C, 0x01] → mutable f64 global
      def read_global_type
        vt_offset = @pos
        value_type_byte = read_byte
        unless valid_value_type?(value_type_byte)
          raise WasmParseError.new(
            "Unknown value type byte 0x#{value_type_byte.to_s(16)} at offset #{vt_offset}",
            vt_offset
          )
        end
        mutable_byte = read_byte
        GlobalType.new(value_type_byte, mutable_byte != 0)
      end

      # read_init_expr — read bytes until and including 0x0B (end opcode).
      #
      # Constant init expressions are used to initialize:
      #   - Global variables
      #   - Table offsets (element section)
      #   - Memory offsets (data section)
      #
      # They are sequences of opcodes ending with 0x0B. We collect all bytes
      # as a binary String, including the 0x0B.
      #
      # Common init expressions (as bytes):
      #   [0x41, 0x00, 0x0B]  → i32.const 0; end
      #   [0x41, 0x2A, 0x0B]  → i32.const 42; end
      def read_init_expr
        start = @pos
        while @pos < @data.length
          b = @data[@pos]
          @pos += 1
          return @data[start, @pos - start].pack("C*").b if b == END_OPCODE
        end
        raise WasmParseError.new(
          "Init expression at offset #{start} never terminated with 0x0B (end opcode)",
          start
        )
      end

      # ── Section parsers ──────────────────────────────────────────────────

      # parse_type_section — parse section ID 1: function type signatures.
      #
      # The type section lists all unique function signatures in the module.
      # Imported and local functions reference these by index.
      #
      # Structure:
      #   count:u32leb
      #   for each type:
      #     0x60  (function type prefix)
      #     param_count:u32leb
      #     param_types:u8[param_count]
      #     result_count:u32leb
      #     result_types:u8[result_count]
      #
      # Example — (i32, i32) → i32:
      #   [0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F]
      #           ^^^^ ^^^^ ^^^^  ^^^^ ^^^^
      #           2    i32  i32  1    i32
      def parse_type_section(mod)
        count = read_u32
        count.times do
          prefix_offset = @pos
          prefix = read_byte
          unless prefix == FUNC_TYPE_PREFIX
            raise WasmParseError.new(
              "Expected function type prefix 0x60 at offset #{prefix_offset}, " \
              "got 0x#{prefix.to_s(16)}",
              prefix_offset
            )
          end
          params  = read_value_type_vec
          results = read_value_type_vec
          mod.types << FuncType.new(params, results)
        end
      end

      # read_value_type_vec — read a vector of value types: count:u32leb + type_byte[count].
      #
      # Value type bytes:
      #   0x7F = i32  (32-bit integer)
      #   0x7E = i64  (64-bit integer)
      #   0x7D = f32  (32-bit float)
      #   0x7C = f64  (64-bit float)
      def read_value_type_vec
        count = read_u32
        types = []
        count.times do
          vt_offset = @pos
          b = read_byte
          unless valid_value_type?(b)
            raise WasmParseError.new(
              "Unknown value type byte 0x#{b.to_s(16)} at offset #{vt_offset}",
              vt_offset
            )
          end
          types << b
        end
        types
      end

      # parse_import_section — parse section ID 2: host imports.
      #
      # Imports let a module request functions, tables, memories, and globals
      # from the host environment.
      #
      # Structure per import:
      #   module_name:str  (e.g. "env")
      #   name:str         (e.g. "memory")
      #   kind:u8          (0=func, 1=table, 2=memory, 3=global)
      #   type_desc        (depends on kind)
      def parse_import_section(mod)
        count = read_u32
        count.times do
          module_name = read_string
          name        = read_string
          kind_offset = @pos
          kind_byte   = read_byte

          type_info = case kind_byte
                      when EXTERNAL_KIND[:function]
                        read_u32  # type section index
                      when EXTERNAL_KIND[:table]
                        et_offset = @pos
                        element_type = read_byte
                        unless element_type == FUNCREF
                          raise WasmParseError.new(
                            "Unknown table element type 0x#{element_type.to_s(16)} at offset #{et_offset}",
                            et_offset
                          )
                        end
                        TableType.new(element_type, read_limits)
                      when EXTERNAL_KIND[:memory]
                        MemoryType.new(read_limits)
                      when EXTERNAL_KIND[:global]
                        read_global_type
                      else
                        raise WasmParseError.new(
                          "Unknown import kind 0x#{kind_byte.to_s(16)} at offset #{kind_offset}",
                          kind_offset
                        )
                      end

          # Map byte to symbol for kind
          kind_sym = EXTERNAL_KIND.key(kind_byte)
          mod.imports << Import.new(module_name, name, kind_sym, type_info)
        end
      end

      # parse_function_section — parse section ID 3: type indices for local functions.
      #
      # One type-section index per locally-defined function. Code is in the code section.
      #
      # Structure:
      #   count:u32leb
      #   [type_index:u32leb × count]
      def parse_function_section(mod)
        count = read_u32
        count.times { mod.functions << read_u32 }
      end

      # parse_table_section — parse section ID 4: indirect call tables.
      #
      # Tables are indexed arrays of function references. WASM 1.0 allows
      # only funcref (0x70) as the element type.
      #
      # Structure per table:
      #   element_type:u8 (always 0x70 = funcref in WASM 1.0)
      #   limits
      def parse_table_section(mod)
        count = read_u32
        count.times do
          et_offset    = @pos
          element_type = read_byte
          unless element_type == FUNCREF
            raise WasmParseError.new(
              "Unknown table element type 0x#{element_type.to_s(16)} at offset #{et_offset}",
              et_offset
            )
          end
          mod.tables << TableType.new(element_type, read_limits)
        end
      end

      # parse_memory_section — parse section ID 5: linear memory declarations.
      #
      # Linear memory is the WASM heap — a flat, resizable byte array.
      # WASM 1.0 allows at most one memory per module. Size is in 64-KiB pages.
      #
      # Structure per memory:
      #   limits  (min pages, optional max pages)
      def parse_memory_section(mod)
        count = read_u32
        count.times { mod.memories << MemoryType.new(read_limits) }
      end

      # parse_global_section — parse section ID 6: module-level global variables.
      #
      # Each global has a type (value type + mutability) and a constant init
      # expression that sets its initial value.
      #
      # Structure per global:
      #   globaltype
      #   init_expr (bytes until 0x0B inclusive)
      def parse_global_section(mod)
        count = read_u32
        count.times do
          global_type = read_global_type
          init_expr   = read_init_expr
          mod.globals << Global.new(global_type, init_expr)
        end
      end

      # parse_export_section — parse section ID 7: host-visible exports.
      #
      # Exports make module internals visible to the host environment.
      #
      # Structure per export:
      #   name:str
      #   kind:u8  (0=func, 1=table, 2=memory, 3=global)
      #   index:u32leb
      def parse_export_section(mod)
        count = read_u32
        count.times do
          name        = read_string
          kind_offset = @pos
          kind_byte   = read_byte
          kind_sym    = EXTERNAL_KIND.key(kind_byte)
          unless kind_sym
            raise WasmParseError.new(
              "Unknown export kind 0x#{kind_byte.to_s(16)} at offset #{kind_offset}",
              kind_offset
            )
          end
          index = read_u32
          mod.exports << Export.new(name, kind_sym, index)
        end
      end

      # parse_start_section — parse section ID 8: the entry-point function index.
      #
      # If present, the runtime calls this function automatically after
      # instantiation, before any exports are callable.
      #
      # Structure:
      #   function_index:u32leb
      def parse_start_section(mod)
        mod.start = read_u32
      end

      # parse_element_section — parse section ID 9: table initializers.
      #
      # Element segments populate a table with function references at
      # instantiation time. This enables call_indirect to find functions.
      #
      # Structure per element:
      #   table_index:u32leb
      #   offset_expr (constant expression = starting slot)
      #   func_count:u32leb
      #   func_indices:u32leb[func_count]
      def parse_element_section(mod)
        count = read_u32
        count.times do
          table_index      = read_u32
          offset_expr      = read_init_expr
          func_count       = read_u32
          function_indices = func_count.times.map { read_u32 }
          mod.elements << Element.new(table_index, offset_expr, function_indices)
        end
      end

      # parse_code_section — parse section ID 10: function bodies.
      #
      # One function body per locally-defined function. Each body has:
      #   - Local variable declarations (run-length encoded)
      #   - Raw opcode bytes
      #
      # Structure per body:
      #   body_size:u32leb            (total byte length of the body)
      #   local_decl_count:u32leb     (number of local groups)
      #   for each local group:
      #     group_count:u32leb        (how many locals of this type)
      #     type:u8                   (the ValueType of those locals)
      #   code:remaining_bytes_in_body (raw opcodes, ends with 0x0B)
      #
      # Local groups are run-length encoded for compactness. We expand:
      # "(3, i32)" → [i32, i32, i32].
      #
      # Example body:
      #   [0x01]        ← 1 local group
      #   [0x02, 0x7F]  ← 2 × i32
      #   [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B]  ← code
      def parse_code_section(mod)
        count = read_u32
        count.times do |i|
          body_size  = read_u32
          body_start = @pos
          body_end   = body_start + body_size

          if body_end > @data.length
            raise WasmParseError.new(
              "Code body #{i} extends beyond end of data (offset #{body_start}, size #{body_size})",
              body_start
            )
          end

          # Read local declarations (run-length encoded groups)
          local_decl_count = read_u32
          locals = []
          local_decl_count.times do
            group_count = read_u32
            vt_offset   = @pos
            type_byte   = read_byte
            unless valid_value_type?(type_byte)
              raise WasmParseError.new(
                "Unknown local type byte 0x#{type_byte.to_s(16)} at offset #{vt_offset}",
                vt_offset
              )
            end
            # Expand the run-length group
            group_count.times { locals << type_byte }
          end

          # The remaining bytes are the raw code
          code_length = body_end - @pos
          if code_length < 0
            raise WasmParseError.new(
              "Code body #{i} local declarations exceeded body size at offset #{@pos}",
              @pos
            )
          end
          code = read_bytes(code_length).pack("C*").b
          mod.code << FunctionBody.new(locals, code)
        end
      end

      # parse_data_section — parse section ID 11: memory initializers.
      #
      # Data segments copy raw bytes into linear memory at instantiation.
      # Think of it like the .data / .rodata sections in an ELF binary.
      #
      # Structure per segment:
      #   mem_index:u32leb   (which memory; always 0 in WASM 1.0)
      #   offset_expr        (constant expression = byte address in memory)
      #   byte_count:u32leb
      #   data:u8[byte_count]
      def parse_data_section(mod)
        count = read_u32
        count.times do
          memory_index = read_u32
          offset_expr  = read_init_expr
          byte_count   = read_u32
          data         = read_bytes(byte_count).pack("C*").b
          mod.data << DataSegment.new(memory_index, offset_expr, data)
        end
      end

      # parse_custom_section — parse section ID 0: named byte blobs.
      #
      # Custom sections carry metadata that doesn't affect execution.
      # Well-known custom sections:
      #   "name"      — debug names for functions, locals, globals
      #   "producers" — compiler/toolchain metadata
      #
      # Structure:
      #   name:str          (section name)
      #   data:remaining    (raw bytes up to section boundary)
      #
      # We receive a sub-slice of @data (the payload), so we create a
      # sub-reader over it to safely parse name + remaining bytes.
      def parse_custom_section(mod, payload_bytes)
        sub = BinaryReader.new(payload_bytes)
        name = sub.read_string
        data = sub.read_bytes(payload_bytes.length - sub.pos).pack("C*").b
        mod.customs << CustomSection.new(name, data)
      end

      # ── Top-level module parsing ────────────────────────────────────────

      # parse_module — read and validate the header, then dispatch sections.
      #
      # The header is always 8 bytes:
      #   Bytes 0–3: magic "\0asm"
      #   Bytes 4–7: version 1 (little-endian u32)
      #
      # After the header, we loop reading sections until end-of-input.
      # Each section is: id:u8 + size:u32leb + payload:size_bytes.
      #
      # We track last_section_id to enforce ordering (1–11 must be ascending).
      # Custom sections (id=0) are exempt from ordering.
      def parse_module
        validate_header
        mod = WasmModule.new
        last_section_id = 0

        until at_end?
          section_id_offset = @pos
          section_id        = read_byte
          payload_size      = read_u32
          payload_start     = @pos
          payload_end       = payload_start + payload_size

          if payload_end > @data.length
            raise WasmParseError.new(
              "Section #{section_id} payload extends beyond end of data " \
              "(offset #{payload_start}, size #{payload_size})",
              payload_start
            )
          end

          # Enforce ordering for numbered sections (1–11).
          # Custom sections (0) may appear anywhere.
          if section_id != SECTION_CUSTOM
            if section_id < last_section_id
              raise WasmParseError.new(
                "Section #{section_id} appears out of order: already saw section #{last_section_id}",
                section_id_offset
              )
            end
            last_section_id = section_id
          end

          # Extract payload as a slice for custom-section parsing
          payload_bytes = @data[payload_start, payload_size]

          case section_id
          when SECTION_TYPE     then parse_type_section(mod)
          when SECTION_IMPORT   then parse_import_section(mod)
          when SECTION_FUNCTION then parse_function_section(mod)
          when SECTION_TABLE    then parse_table_section(mod)
          when SECTION_MEMORY   then parse_memory_section(mod)
          when SECTION_GLOBAL   then parse_global_section(mod)
          when SECTION_EXPORT   then parse_export_section(mod)
          when SECTION_START    then parse_start_section(mod)
          when SECTION_ELEMENT  then parse_element_section(mod)
          when SECTION_CODE     then parse_code_section(mod)
          when SECTION_DATA     then parse_data_section(mod)
          when SECTION_CUSTOM   then parse_custom_section(mod, payload_bytes)
          # Unknown section — skip silently (forward-compatibility).
          end

          # Always advance to the next section boundary.
          @pos = payload_end
        end

        mod
      end

      private

      # validate_header — check magic bytes and version number.
      #
      # If these 8 bytes are wrong, the data is not a WASM module at all.
      # Common causes of failure:
      #   - Passing a .wat (text format) file instead of a .wasm binary
      #   - Truncated download (file cut short)
      #   - Byte-order error
      def validate_header
        if @data.length < 8
          raise WasmParseError.new(
            "File too short: #{@data.length} bytes (need at least 8 for the header)",
            0
          )
        end

        # Check magic bytes: \0asm
        WASM_MAGIC.each_with_index do |expected, i|
          actual = @data[i]
          unless actual == expected
            raise WasmParseError.new(
              "Invalid magic bytes at offset #{i}: " \
              "expected 0x#{expected.to_s(16).rjust(2, "0")}, " \
              "got 0x#{actual.to_s(16).rjust(2, "0")}",
              i
            )
          end
        end
        @pos = 4

        # Check version: [0x01, 0x00, 0x00, 0x00]
        WASM_VERSION.each_with_index do |expected, i|
          actual = @data[4 + i]
          unless actual == expected
            raise WasmParseError.new(
              "Unsupported WASM version at offset #{4 + i}: " \
              "expected 0x#{expected.to_s(16).rjust(2, "0")}, " \
              "got 0x#{actual.to_s(16).rjust(2, "0")}",
              4 + i
            )
          end
        end
        @pos = 8
      end

      # valid_value_type? — check if a byte is a valid WASM 1.0 value type.
      #
      # The four value types and their byte codes:
      #
      #   ┌──────┬────────┐
      #   │ Type │ Byte   │
      #   ├──────┼────────┤
      #   │ f64  │ 0x7C   │
      #   │ f32  │ 0x7D   │
      #   │ i64  │ 0x7E   │
      #   │ i32  │ 0x7F   │
      #   └──────┴────────┘
      def valid_value_type?(b)
        b == VALUE_TYPE[:i32] ||
          b == VALUE_TYPE[:i64] ||
          b == VALUE_TYPE[:f32] ||
          b == VALUE_TYPE[:f64]
      end
    end
  end
end
