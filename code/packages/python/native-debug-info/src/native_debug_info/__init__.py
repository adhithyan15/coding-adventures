"""native-debug-info — DWARF 4 and CodeView 4 emission for AOT binaries.

Public API::

    from native_debug_info import DwarfEmitter, CodeViewEmitter, embed_debug_info
"""

from native_debug_info.codeview import CodeViewEmitter
from native_debug_info.dwarf import DwarfEmitter
from native_debug_info.embed import embed_debug_info
from native_debug_info.leb128 import decode_sleb128, decode_uleb128, encode_sleb128, encode_uleb128

__all__ = [
    "DwarfEmitter",
    "CodeViewEmitter",
    "embed_debug_info",
    "encode_uleb128",
    "encode_sleb128",
    "decode_uleb128",
    "decode_sleb128",
]
