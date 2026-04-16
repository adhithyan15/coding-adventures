"""Version-aware metadata for the external generic BEAM instruction set."""

from __future__ import annotations

from dataclasses import dataclass
from functools import cached_property

BEAM_FORMAT_NUMBER = 0


@dataclass(frozen=True)
class BeamOpcode:
    """Metadata for one external generic BEAM instruction."""

    number: int
    name: str
    arity: int
    deprecated: bool = False
    category: str = "Core"


@dataclass(frozen=True)
class BeamProfile:
    """A BEAM compatibility profile keyed to an OTP release family."""

    name: str
    beam_format_number: int
    max_external_opcode: int
    atom_encoding: str
    literal_encoding: str
    supported_chunks: frozenset[str]

    @cached_property
    def opcodes(self) -> tuple[BeamOpcode, ...]:
        """Return the opcodes available in this profile."""
        return tuple(op for op in _ALL_OPCODES if op.number <= self.max_external_opcode)

    @cached_property
    def _by_number(self) -> dict[int, BeamOpcode]:
        return {op.number: op for op in self.opcodes}

    @cached_property
    def _by_name(self) -> dict[str, BeamOpcode]:
        return {op.name: op for op in self.opcodes}

    def opcode_by_number(self, number: int) -> BeamOpcode:
        """Look up an opcode by numeric value."""
        try:
            return self._by_number[number]
        except KeyError as exc:  # pragma: no cover - tiny branch
            msg = f"Opcode {number} is not available in profile {self.name!r}"
            raise KeyError(msg) from exc

    def opcode_by_name(self, name: str) -> BeamOpcode:
        """Look up an opcode by mnemonic."""
        try:
            return self._by_name[name]
        except KeyError as exc:  # pragma: no cover - tiny branch
            msg = f"Opcode {name!r} is not available in profile {self.name!r}"
            raise KeyError(msg) from exc

    def supports_opcode(self, number: int) -> bool:
        """Return True if the opcode number exists in this profile."""
        return number in self._by_number


_OPCODE_ROWS = (
    (1, "label", 1, False, "Core"),
    (2, "func_info", 3, False, "Core"),
    (3, "int_code_end", 0, False, "Core"),
    (4, "call", 2, False, "Function and BIF calls"),
    (5, "call_last", 3, False, "Function and BIF calls"),
    (6, "call_only", 2, False, "Function and BIF calls"),
    (7, "call_ext", 2, False, "Function and BIF calls"),
    (8, "call_ext_last", 3, False, "Function and BIF calls"),
    (9, "bif0", 2, False, "Function and BIF calls"),
    (10, "bif1", 4, False, "Function and BIF calls"),
    (11, "bif2", 5, False, "Function and BIF calls"),
    (12, "allocate", 2, False, "Allocating, deallocating and returning"),
    (13, "allocate_heap", 3, False, "Allocating, deallocating and returning"),
    (14, "allocate_zero", 2, True, "Allocating, deallocating and returning"),
    (15, "allocate_heap_zero", 3, True, "Allocating, deallocating and returning"),
    (16, "test_heap", 2, False, "Allocating, deallocating and returning"),
    (17, "init", 1, True, "Allocating, deallocating and returning"),
    (18, "deallocate", 1, False, "Allocating, deallocating and returning"),
    (19, "return", 0, False, "Allocating, deallocating and returning"),
    (20, "send", 0, False, "Sending & receiving"),
    (21, "remove_message", 0, False, "Sending & receiving"),
    (22, "timeout", 0, False, "Sending & receiving"),
    (23, "loop_rec", 2, False, "Sending & receiving"),
    (24, "loop_rec_end", 1, False, "Sending & receiving"),
    (25, "wait", 1, False, "Sending & receiving"),
    (26, "wait_timeout", 2, False, "Sending & receiving"),
    (27, "m_plus", 4, True, "Arithmetic opcodes"),
    (28, "m_minus", 4, True, "Arithmetic opcodes"),
    (29, "m_times", 4, True, "Arithmetic opcodes"),
    (30, "m_div", 4, True, "Arithmetic opcodes"),
    (31, "int_div", 4, True, "Arithmetic opcodes"),
    (32, "int_rem", 4, True, "Arithmetic opcodes"),
    (33, "int_band", 4, True, "Arithmetic opcodes"),
    (34, "int_bor", 4, True, "Arithmetic opcodes"),
    (35, "int_bxor", 4, True, "Arithmetic opcodes"),
    (36, "int_bsl", 4, True, "Arithmetic opcodes"),
    (37, "int_bsr", 4, True, "Arithmetic opcodes"),
    (38, "int_bnot", 3, True, "Arithmetic opcodes"),
    (39, "is_lt", 3, False, "Comparison operators"),
    (40, "is_ge", 3, False, "Comparison operators"),
    (41, "is_eq", 3, False, "Comparison operators"),
    (42, "is_ne", 3, False, "Comparison operators"),
    (43, "is_eq_exact", 3, False, "Comparison operators"),
    (44, "is_ne_exact", 3, False, "Comparison operators"),
    (45, "is_integer", 2, False, "Type tests"),
    (46, "is_float", 2, False, "Type tests"),
    (47, "is_number", 2, False, "Type tests"),
    (48, "is_atom", 2, False, "Type tests"),
    (49, "is_pid", 2, False, "Type tests"),
    (50, "is_reference", 2, False, "Type tests"),
    (51, "is_port", 2, False, "Type tests"),
    (52, "is_nil", 2, False, "Type tests"),
    (53, "is_binary", 2, False, "Type tests"),
    (54, "is_constant", 2, True, "Type tests"),
    (55, "is_list", 2, False, "Type tests"),
    (56, "is_nonempty_list", 2, False, "Type tests"),
    (57, "is_tuple", 2, False, "Type tests"),
    (58, "test_arity", 3, False, "Type tests"),
    (59, "select_val", 3, False, "Indexing & jumping"),
    (60, "select_tuple_arity", 3, False, "Indexing & jumping"),
    (61, "jump", 1, False, "Indexing & jumping"),
    (62, "catch", 2, False, "Catch"),
    (63, "catch_end", 1, False, "Catch"),
    (64, "move", 2, False, "Moving, extracting, modifying"),
    (65, "get_list", 3, False, "Moving, extracting, modifying"),
    (66, "get_tuple_element", 3, False, "Moving, extracting, modifying"),
    (67, "set_tuple_element", 3, False, "Moving, extracting, modifying"),
    (68, "put_string", 3, True, "Building terms"),
    (69, "put_list", 3, False, "Building terms"),
    (70, "put_tuple", 2, True, "Building terms"),
    (71, "put", 1, True, "Building terms"),
    (72, "badmatch", 1, False, "Raising errors"),
    (73, "if_end", 0, False, "Raising errors"),
    (74, "case_end", 1, False, "Raising errors"),
    (75, "call_fun", 1, False, "Raising errors"),
    (76, "make_fun", 3, True, "Raising errors"),
    (77, "is_function", 2, False, "Raising errors"),
    (78, "call_ext_only", 2, False, "Late additions to R5"),
    (79, "bs_start_match", 2, True, "Binary matching"),
    (80, "bs_get_integer", 5, True, "Binary matching"),
    (81, "bs_get_float", 5, True, "Binary matching"),
    (82, "bs_get_binary", 5, True, "Binary matching"),
    (83, "bs_skip_bits", 4, True, "Binary matching"),
    (84, "bs_test_tail", 2, True, "Binary matching"),
    (85, "bs_save", 1, True, "Binary matching"),
    (86, "bs_restore", 1, True, "Binary matching"),
    (87, "bs_init", 2, True, "Binary construction"),
    (88, "bs_final", 2, True, "Binary construction"),
    (89, "bs_put_integer", 5, True, "Binary construction"),
    (90, "bs_put_binary", 5, True, "Binary construction"),
    (91, "bs_put_float", 5, True, "Binary construction"),
    (92, "bs_put_string", 2, True, "Binary construction"),
    (93, "bs_need_buf", 1, True, "Binary construction"),
    (94, "fclearerror", 0, True, "Floating point"),
    (95, "fcheckerror", 1, True, "Floating point"),
    (96, "fmove", 2, False, "Floating point"),
    (97, "fconv", 2, False, "Floating point"),
    (98, "fadd", 4, False, "Floating point"),
    (99, "fsub", 4, False, "Floating point"),
    (100, "fmul", 4, False, "Floating point"),
    (101, "fdiv", 4, False, "Floating point"),
    (102, "fnegate", 3, False, "Floating point"),
    (103, "make_fun2", 1, True, "New fun construction"),
    (104, "try", 2, False, "Try/catch/raise"),
    (105, "try_end", 1, False, "Try/catch/raise"),
    (106, "try_case", 1, False, "Try/catch/raise"),
    (107, "try_case_end", 1, False, "Try/catch/raise"),
    (108, "raise", 2, False, "Try/catch/raise"),
    (109, "bs_init2", 6, True, "R10B additions"),
    (110, "bs_bits_to_bytes", 3, True, "R10B additions"),
    (111, "bs_add", 5, True, "R10B additions"),
    (112, "apply", 1, False, "R10B additions"),
    (113, "apply_last", 2, False, "R10B additions"),
    (114, "is_boolean", 2, False, "R10B additions"),
    (115, "is_function2", 3, False, "R10B-6"),
    (116, "bs_start_match2", 5, True, "R11B bit syntax"),
    (117, "bs_get_integer2", 7, False, "R11B bit syntax"),
    (118, "bs_get_float2", 7, False, "R11B bit syntax"),
    (119, "bs_get_binary2", 7, False, "R11B bit syntax"),
    (120, "bs_skip_bits2", 5, False, "R11B bit syntax"),
    (121, "bs_test_tail2", 3, False, "R11B bit syntax"),
    (122, "bs_save2", 2, True, "R11B bit syntax"),
    (123, "bs_restore2", 2, True, "R11B bit syntax"),
    (124, "gc_bif1", 5, False, "GC BIFs"),
    (125, "gc_bif2", 6, False, "GC BIFs"),
    (126, "bs_final2", 2, True, "Unused"),
    (127, "bs_bits_to_bytes2", 2, True, "Unused"),
    (128, "put_literal", 2, True, "R11B-4"),
    (129, "is_bitstr", 2, False, "R11B-5"),
    (130, "bs_context_to_binary", 1, True, "R12B"),
    (131, "bs_test_unit", 3, True, "R12B"),
    (132, "bs_match_string", 4, False, "R12B"),
    (133, "bs_init_writable", 0, False, "R12B"),
    (134, "bs_append", 8, True, "R12B"),
    (135, "bs_private_append", 6, True, "R12B"),
    (136, "trim", 2, False, "R12B"),
    (137, "bs_init_bits", 6, True, "R12B"),
    (138, "bs_get_utf8", 5, False, "UTF support"),
    (139, "bs_skip_utf8", 4, False, "UTF support"),
    (140, "bs_get_utf16", 5, False, "UTF support"),
    (141, "bs_skip_utf16", 4, False, "UTF support"),
    (142, "bs_get_utf32", 5, False, "UTF support"),
    (143, "bs_skip_utf32", 4, False, "UTF support"),
    (144, "bs_utf8_size", 3, True, "UTF support"),
    (145, "bs_put_utf8", 3, True, "UTF support"),
    (146, "bs_utf16_size", 3, True, "UTF support"),
    (147, "bs_put_utf16", 3, True, "UTF support"),
    (148, "bs_put_utf32", 3, True, "UTF support"),
    (149, "on_load", 0, False, "R13B03"),
    (150, "recv_mark", 1, True, "R14A"),
    (151, "recv_set", 1, True, "R14A"),
    (152, "gc_bif3", 7, False, "R14A"),
    (153, "line", 1, False, "R15A"),
    (154, "put_map_assoc", 5, False, "Maps"),
    (155, "put_map_exact", 5, False, "Maps"),
    (156, "is_map", 2, False, "Maps"),
    (157, "has_map_fields", 3, False, "Maps"),
    (158, "get_map_elements", 3, False, "Maps"),
    (159, "is_tagged_tuple", 4, False, "OTP 20"),
    (160, "build_stacktrace", 0, False, "OTP 21"),
    (161, "raw_raise", 0, False, "OTP 21"),
    (162, "get_hd", 2, False, "OTP 21"),
    (163, "get_tl", 2, False, "OTP 21"),
    (164, "put_tuple2", 2, False, "OTP 22"),
    (165, "bs_get_tail", 3, False, "OTP 22"),
    (166, "bs_start_match3", 4, False, "OTP 22"),
    (167, "bs_get_position", 3, False, "OTP 22"),
    (168, "bs_set_position", 2, False, "OTP 22"),
    (169, "swap", 2, False, "OTP 23"),
    (170, "bs_start_match4", 4, False, "OTP 23"),
    (171, "make_fun3", 3, False, "OTP 24"),
    (172, "init_yregs", 1, False, "OTP 24"),
    (173, "recv_marker_bind", 2, False, "OTP 24"),
    (174, "recv_marker_clear", 1, False, "OTP 24"),
    (175, "recv_marker_reserve", 1, False, "OTP 24"),
    (176, "recv_marker_use", 1, False, "OTP 24"),
    (177, "bs_create_bin", 6, False, "OTP 25"),
    (178, "call_fun2", 3, False, "OTP 25"),
    (179, "nif_start", 0, False, "OTP 25"),
    (180, "badrecord", 1, False, "OTP 25"),
    (181, "update_record", 5, False, "OTP 26"),
    (182, "bs_match", 3, False, "OTP 26"),
    (183, "executable_line", 2, False, "OTP 27"),
    (184, "debug_line", 4, False, "OTP 28"),
    (185, "bif3", 6, False, "OTP 29"),
    (186, "is_any_native_record", 2, False, "OTP 29"),
    (187, "is_native_record", 4, False, "OTP 29"),
    (188, "get_record_elements", 3, False, "OTP 29"),
    (189, "put_record", 6, False, "OTP 29"),
    (190, "is_record_accessible", 3, False, "OTP 29"),
    (191, "get_record_field", 5, False, "OTP 29"),
)

_ALL_OPCODES = tuple(BeamOpcode(*row) for row in _OPCODE_ROWS)

OTP_24_PROFILE = BeamProfile(
    name="otp24",
    beam_format_number=BEAM_FORMAT_NUMBER,
    max_external_opcode=176,
    atom_encoding="legacy-or-long",
    literal_encoding="compressed",
    supported_chunks=frozenset(
        {
            "AtU8",
            "Code",
            "StrT",
            "ImpT",
            "ExpT",
            "FunT",
            "LitT",
            "Attr",
            "CInf",
            "Line",
            "LocT",
            "Type",
        }
    ),
)

OTP_28_PROFILE = BeamProfile(
    name="otp28",
    beam_format_number=BEAM_FORMAT_NUMBER,
    max_external_opcode=184,
    atom_encoding="long",
    literal_encoding="uncompressed",
    supported_chunks=frozenset(
        {
            "AtU8",
            "Code",
            "StrT",
            "ImpT",
            "ExpT",
            "FunT",
            "LitT",
            "Attr",
            "CInf",
            "Line",
            "LocT",
            "Type",
            "Meta",
            "DbgB",
            "Recs",
        }
    ),
)

OTP_29_PROFILE = BeamProfile(
    name="otp29",
    beam_format_number=BEAM_FORMAT_NUMBER,
    max_external_opcode=191,
    atom_encoding="long",
    literal_encoding="uncompressed",
    supported_chunks=OTP_28_PROFILE.supported_chunks,
)

_PROFILES = {
    OTP_24_PROFILE.name: OTP_24_PROFILE,
    OTP_28_PROFILE.name: OTP_28_PROFILE,
    OTP_29_PROFILE.name: OTP_29_PROFILE,
}


def get_profile(name: str) -> BeamProfile:
    """Return a BEAM profile by name."""
    try:
        return _PROFILES[name]
    except KeyError as exc:  # pragma: no cover - tiny branch
        msg = f"Unknown BEAM profile {name!r}"
        raise KeyError(msg) from exc


def list_profiles() -> tuple[BeamProfile, ...]:
    """Return all built-in BEAM profiles."""
    return tuple(_PROFILES.values())
