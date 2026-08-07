/*
 * board_vm_protocol.c — implementation of the board VM wire protocol codec.
 *
 * A faithful, allocation-free port of the Rust `board-vm-protocol` crate.
 * Every routine mirrors its Rust counterpart's control flow and bounds checks
 * exactly; see board_vm_protocol.h for the surface documentation.
 *
 * All multi-byte integers are little-endian on the wire, matching Rust's
 * `to_le_bytes` / `from_le_bytes`.
 */
#include "board_vm_protocol.h"

#include <string.h>

/* ================================================================== */
/* Golden test vectors                                                */
/* ================================================================== */

const uint8_t BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1[10] = {
    0x01, 0x01, 0x03, 'b', 'v', 'm', 0xCD, 0xAB, 0x34, 0x12
};
const uint8_t BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1[18] = {
    0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b', 'v', 'm',
    0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49
};
const uint8_t BVM_GOLDEN_HELLO_WIRE_FRAME_BVM_V1[20] = {
    0x13, 0x01, 0x01, 0x01, 0x34, 0x12, 0x0A, 0x01, 0x01, 0x03, 'b', 'v', 'm',
    0xCD, 0xAB, 0x34, 0x12, 0x19, 0x49, 0x00
};
const uint8_t BVM_GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1[11] = {
    0x01, 0x00, 0x01, 0x24, 0x00, 0x00, 0x00, 0xBE, 0xBA, 0xFE, 0xCA
};
const uint8_t BVM_GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1[11] = {
    0x01, 0x00, 0x05, 0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
};

/* ================================================================== */
/* Small enumerations                                                 */
/* ================================================================== */

int bvm_message_type_is_vendor_extension(uint8_t message_type) {
    return message_type >= 0x80u;
}

bvm_error_t bvm_program_format_validate(uint8_t value) {
    if (value == BVM_PROGRAM_FORMAT_BVM_MODULE) {
        return BVM_OK;
    }
    return BVM_ERR_UNSUPPORTED_VALUE;
}

bvm_error_t bvm_run_status_validate(uint8_t value) {
    switch (value) {
    case BVM_RUN_STATUS_HALTED:
    case BVM_RUN_STATUS_RUNNING:
    case BVM_RUN_STATUS_STOPPED:
    case BVM_RUN_STATUS_BUDGET_EXCEEDED:
    case BVM_RUN_STATUS_FAULTED:
        return BVM_OK;
    default:
        return BVM_ERR_UNSUPPORTED_VALUE;
    }
}

/* ================================================================== */
/* UTF-8 validation                                                   */
/* ================================================================== */

/*
 * Rust's `str::from_utf8` rejects any byte sequence that is not well-formed
 * UTF-8.  We replicate that here so decoded strings uphold the same invariant
 * the Rust crate guarantees.  The rules (RFC 3629):
 *
 *   0xxxxxxx                              — 1 byte, U+0000..U+007F
 *   110xxxxx 10xxxxxx                     — 2 bytes, U+0080..U+07FF
 *   1110xxxx 10xxxxxx 10xxxxxx            — 3 bytes, U+0800..U+FFFF
 *   11110xxx 10xxxxxx 10xxxxxx 10xxxxxx   — 4 bytes, U+10000..U+10FFFF
 *
 * Over-long encodings and surrogates (U+D800..U+DFFF) are rejected via the
 * lower-bound checks on the leading continuation byte.
 */
static int bvm_is_utf8(const uint8_t *s, size_t len) {
    size_t i = 0;
    while (i < len) {
        uint8_t b0 = s[i];
        if (b0 < 0x80u) {
            i += 1;
        } else if ((b0 & 0xE0u) == 0xC0u) {
            /* 2-byte sequence; reject overlong (< 0xC2). */
            if (b0 < 0xC2u) {
                return 0;
            }
            if (i + 1 >= len || (s[i + 1] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 2;
        } else if ((b0 & 0xF0u) == 0xE0u) {
            uint8_t b1;
            if (i + 2 >= len) {
                return 0;
            }
            b1 = s[i + 1];
            if ((b1 & 0xC0u) != 0x80u || (s[i + 2] & 0xC0u) != 0x80u) {
                return 0;
            }
            /* Reject overlong (E0 80..9F) and surrogates (ED A0..BF). */
            if (b0 == 0xE0u && b1 < 0xA0u) {
                return 0;
            }
            if (b0 == 0xEDu && b1 >= 0xA0u) {
                return 0;
            }
            i += 3;
        } else if ((b0 & 0xF8u) == 0xF0u) {
            uint8_t b1;
            if (b0 > 0xF4u) {
                return 0;
            }
            if (i + 3 >= len) {
                return 0;
            }
            b1 = s[i + 1];
            if ((b1 & 0xC0u) != 0x80u || (s[i + 2] & 0xC0u) != 0x80u ||
                (s[i + 3] & 0xC0u) != 0x80u) {
                return 0;
            }
            /* Reject overlong (F0 80..8F) and > U+10FFFF (F4 90..BF). */
            if (b0 == 0xF0u && b1 < 0x90u) {
                return 0;
            }
            if (b0 == 0xF4u && b1 >= 0x90u) {
                return 0;
            }
            i += 4;
        } else {
            return 0;
        }
    }
    return 1;
}

/* ================================================================== */
/* Encoder                                                            */
/* ================================================================== */

bvm_encoder_t bvm_encoder_new(uint8_t *out, size_t cap) {
    bvm_encoder_t enc;
    enc.out = out;
    enc.cap = cap;
    enc.len = 0;
    return enc;
}

size_t bvm_encoder_len(const bvm_encoder_t *enc) {
    return enc->len;
}

int bvm_encoder_is_empty(const bvm_encoder_t *enc) {
    return enc->len == 0;
}

bvm_error_t bvm_encoder_write_slice(bvm_encoder_t *enc, const uint8_t *value, size_t len) {
    /* Rust checks len.checked_add(value.len()) > out.len().  Because the
     * invariant enc->len <= enc->cap always holds, (cap - len) cannot
     * underflow, so testing value_len against the remaining room is both
     * overflow-safe and equivalent. */
    if (len > enc->cap - enc->len) {
        return BVM_ERR_OUTPUT_TOO_SMALL;
    }
    if (len != 0) {
        memcpy(enc->out + enc->len, value, len);
    }
    enc->len += len;
    return BVM_OK;
}

bvm_error_t bvm_encoder_write_u8(bvm_encoder_t *enc, uint8_t value) {
    return bvm_encoder_write_slice(enc, &value, 1);
}

bvm_error_t bvm_encoder_write_bool(bvm_encoder_t *enc, int value) {
    return bvm_encoder_write_u8(enc, value ? 1u : 0u);
}

bvm_error_t bvm_encoder_write_u16(bvm_encoder_t *enc, uint16_t value) {
    uint8_t buf[2];
    buf[0] = (uint8_t)(value & 0xFFu);
    buf[1] = (uint8_t)((value >> 8) & 0xFFu);
    return bvm_encoder_write_slice(enc, buf, 2);
}

bvm_error_t bvm_encoder_write_u32(bvm_encoder_t *enc, uint32_t value) {
    uint8_t buf[4];
    buf[0] = (uint8_t)(value & 0xFFu);
    buf[1] = (uint8_t)((value >> 8) & 0xFFu);
    buf[2] = (uint8_t)((value >> 16) & 0xFFu);
    buf[3] = (uint8_t)((value >> 24) & 0xFFu);
    return bvm_encoder_write_slice(enc, buf, 4);
}

bvm_error_t bvm_encoder_write_i16(bvm_encoder_t *enc, int16_t value) {
    /* Two's-complement reinterpretation matches Rust's i16::to_le_bytes. */
    return bvm_encoder_write_u16(enc, (uint16_t)value);
}

bvm_error_t bvm_encoder_write_uleb128(bvm_encoder_t *enc, uint32_t value) {
    for (;;) {
        uint8_t byte = (uint8_t)(value & 0x7Fu);
        value >>= 7;
        if (value != 0) {
            byte |= 0x80u;
        }
        {
            bvm_error_t err = bvm_encoder_write_u8(enc, byte);
            if (err != BVM_OK) {
                return err;
            }
        }
        if (value == 0) {
            return BVM_OK;
        }
    }
}

bvm_error_t bvm_encoder_write_bytes(bvm_encoder_t *enc, const uint8_t *value, size_t len) {
    /* On the wire a byte-string is a ULEB128 length prefix + the raw bytes.
     * Rust caps the length at u32::MAX; on a 32-bit size_t this is always
     * true (SIZE_MAX == UINT32_MAX) so the comparison is correct either way. */
    bvm_error_t err;
    if (len > (size_t)UINT32_MAX) {
        return BVM_ERR_PAYLOAD_TOO_LARGE;
    }
    err = bvm_encoder_write_uleb128(enc, (uint32_t)len);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_encoder_write_slice(enc, value, len);
}

bvm_error_t bvm_encoder_write_string(bvm_encoder_t *enc, const char *value, size_t len) {
    return bvm_encoder_write_bytes(enc, (const uint8_t *)value, len);
}

bvm_error_t bvm_encoder_write_capability_descriptor(bvm_encoder_t *enc,
                                                    const bvm_capability_descriptor_t *v) {
    bvm_error_t err;
    err = bvm_encoder_write_u16(enc, v->id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(enc, v->version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u16(enc, v->flags);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_encoder_write_string(enc, v->name, v->name_len);
}

bvm_error_t bvm_encoder_write_value(bvm_encoder_t *enc, const bvm_value_t *v) {
    bvm_error_t err;
    switch (v->tag) {
    case BVM_VALUE_UNIT:
        return bvm_encoder_write_u8(enc, 0x00u);
    case BVM_VALUE_BOOL:
        err = bvm_encoder_write_u8(enc, 0x01u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_bool(enc, v->as.boolean);
    case BVM_VALUE_U8:
        err = bvm_encoder_write_u8(enc, 0x02u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_u8(enc, v->as.u8);
    case BVM_VALUE_U16:
        err = bvm_encoder_write_u8(enc, 0x03u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_u16(enc, v->as.u16);
    case BVM_VALUE_U32:
        err = bvm_encoder_write_u8(enc, 0x04u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_u32(enc, v->as.u32);
    case BVM_VALUE_I16:
        err = bvm_encoder_write_u8(enc, 0x05u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_i16(enc, v->as.i16);
    case BVM_VALUE_HANDLE:
        err = bvm_encoder_write_u8(enc, 0x06u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_u16(enc, v->as.handle);
    case BVM_VALUE_BYTES:
        err = bvm_encoder_write_u8(enc, 0x07u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_bytes(enc, v->as.bytes.ptr, v->as.bytes.len);
    case BVM_VALUE_STRING:
        err = bvm_encoder_write_u8(enc, 0x08u);
        if (err != BVM_OK) {
            return err;
        }
        return bvm_encoder_write_string(enc, v->as.str.ptr, v->as.str.len);
    default:
        /* Unreachable for a well-formed value; treat as a programming error. */
        return BVM_ERR_UNSUPPORTED_VALUE;
    }
}

/* ================================================================== */
/* Decoder                                                            */
/* ================================================================== */

bvm_decoder_t bvm_decoder_new(const uint8_t *input, size_t len) {
    bvm_decoder_t dec;
    dec.input = input;
    dec.input_len = len;
    dec.offset = 0;
    return dec;
}

size_t bvm_decoder_offset(const bvm_decoder_t *dec) {
    return dec->offset;
}

size_t bvm_decoder_remaining_len(const bvm_decoder_t *dec) {
    return dec->input_len - dec->offset;
}

bvm_error_t bvm_decoder_finish(const bvm_decoder_t *dec) {
    if (dec->offset == dec->input_len) {
        return BVM_OK;
    }
    return BVM_ERR_TRAILING_BYTES;
}

bvm_error_t bvm_decoder_read_u8(bvm_decoder_t *dec, uint8_t *out) {
    if (dec->offset >= dec->input_len) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    *out = dec->input[dec->offset];
    dec->offset += 1;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_bool(bvm_decoder_t *dec, int *out) {
    uint8_t b;
    bvm_error_t err = bvm_decoder_read_u8(dec, &b);
    if (err != BVM_OK) {
        return err;
    }
    switch (b) {
    case 0:
        *out = 0;
        return BVM_OK;
    case 1:
        *out = 1;
        return BVM_OK;
    default:
        return BVM_ERR_INVALID_BOOL;
    }
}

/* Read a little-endian u16 at `offset` without advancing; overflow-safe. */
static bvm_error_t bvm_read_le_u16(const uint8_t *bytes, size_t len,
                                   size_t offset, uint16_t *out) {
    if (offset > len || len - offset < 2) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    *out = (uint16_t)((uint16_t)bytes[offset] |
                      ((uint16_t)bytes[offset + 1] << 8));
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_u16(bvm_decoder_t *dec, uint16_t *out) {
    bvm_error_t err = bvm_read_le_u16(dec->input, dec->input_len, dec->offset, out);
    if (err != BVM_OK) {
        return err;
    }
    dec->offset += 2;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_u32(bvm_decoder_t *dec, uint32_t *out) {
    size_t off = dec->offset;
    if (off > dec->input_len || dec->input_len - off < 4) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    *out = (uint32_t)dec->input[off] |
           ((uint32_t)dec->input[off + 1] << 8) |
           ((uint32_t)dec->input[off + 2] << 16) |
           ((uint32_t)dec->input[off + 3] << 24);
    dec->offset += 4;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_i16(bvm_decoder_t *dec, int16_t *out) {
    uint16_t u;
    bvm_error_t err = bvm_decoder_read_u16(dec, &u);
    if (err != BVM_OK) {
        return err;
    }
    *out = (int16_t)u;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_uleb128(bvm_decoder_t *dec, uint32_t *out) {
    uint32_t value = 0;
    unsigned shift = 0;
    for (;;) {
        uint8_t byte;
        uint32_t chunk;
        if (shift >= 35) {
            return BVM_ERR_ULEB_OVERFLOW;
        }
        if (bvm_decoder_read_u8(dec, &byte) != BVM_OK) {
            return BVM_ERR_TRUNCATED_ULEB;
        }
        chunk = (uint32_t)(byte & 0x7Fu);
        if (shift == 28 && chunk > 0x0Fu) {
            return BVM_ERR_ULEB_OVERFLOW;
        }
        value |= chunk << shift;
        if ((byte & 0x80u) == 0) {
            *out = value;
            return BVM_OK;
        }
        shift += 7;
    }
}

bvm_error_t bvm_decoder_read_slice(bvm_decoder_t *dec, size_t len, const uint8_t **out) {
    size_t end;
    /* offset.checked_add(len): overflow -> PayloadTooLarge. */
    if (len > (size_t)-1 - dec->offset) {
        return BVM_ERR_PAYLOAD_TOO_LARGE;
    }
    end = dec->offset + len;
    if (end > dec->input_len) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    *out = dec->input + dec->offset;
    dec->offset = end;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_bytes(bvm_decoder_t *dec, const uint8_t **out, size_t *out_len) {
    uint32_t len;
    bvm_error_t err = bvm_decoder_read_uleb128(dec, &len);
    if (err != BVM_OK) {
        return err;
    }
    *out_len = (size_t)len;
    return bvm_decoder_read_slice(dec, (size_t)len, out);
}

bvm_error_t bvm_decoder_read_string(bvm_decoder_t *dec, const char **out, size_t *out_len) {
    const uint8_t *bytes;
    size_t len;
    bvm_error_t err = bvm_decoder_read_bytes(dec, &bytes, &len);
    if (err != BVM_OK) {
        return err;
    }
    if (!bvm_is_utf8(bytes, len)) {
        return BVM_ERR_INVALID_UTF8;
    }
    *out = (const char *)bytes;
    *out_len = len;
    return BVM_OK;
}

bvm_error_t bvm_decoder_read_capability_descriptor(bvm_decoder_t *dec,
                                                   bvm_capability_descriptor_t *out) {
    bvm_error_t err;
    err = bvm_decoder_read_u16(dec, &out->id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(dec, &out->version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u16(dec, &out->flags);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_read_string(dec, &out->name, &out->name_len);
}

bvm_error_t bvm_decoder_read_value(bvm_decoder_t *dec, bvm_value_t *out) {
    uint8_t tag;
    bvm_error_t err = bvm_decoder_read_u8(dec, &tag);
    if (err != BVM_OK) {
        return err;
    }
    switch (tag) {
    case 0x00:
        out->tag = BVM_VALUE_UNIT;
        return BVM_OK;
    case 0x01:
        out->tag = BVM_VALUE_BOOL;
        return bvm_decoder_read_bool(dec, &out->as.boolean);
    case 0x02:
        out->tag = BVM_VALUE_U8;
        return bvm_decoder_read_u8(dec, &out->as.u8);
    case 0x03:
        out->tag = BVM_VALUE_U16;
        return bvm_decoder_read_u16(dec, &out->as.u16);
    case 0x04:
        out->tag = BVM_VALUE_U32;
        return bvm_decoder_read_u32(dec, &out->as.u32);
    case 0x05:
        out->tag = BVM_VALUE_I16;
        return bvm_decoder_read_i16(dec, &out->as.i16);
    case 0x06:
        out->tag = BVM_VALUE_HANDLE;
        return bvm_decoder_read_u16(dec, &out->as.handle);
    case 0x07:
        out->tag = BVM_VALUE_BYTES;
        return bvm_decoder_read_bytes(dec, &out->as.bytes.ptr, &out->as.bytes.len);
    case 0x08:
        out->tag = BVM_VALUE_STRING;
        return bvm_decoder_read_string(dec, &out->as.str.ptr, &out->as.str.len);
    default:
        return BVM_ERR_UNSUPPORTED_VALUE;
    }
}

/* ================================================================== */
/* Flag / boot-policy validation                                      */
/* ================================================================== */

static bvm_error_t bvm_validate_flags(uint8_t flags) {
    if ((flags & (uint8_t)~BVM_ALLOWED_V1_FLAGS) != 0) {
        return BVM_ERR_RESERVED_FLAGS;
    }
    return BVM_OK;
}

static bvm_error_t bvm_validate_boot_policy(uint8_t value) {
    switch (value) {
    case BVM_BOOT_STORE_ONLY:
    case BVM_BOOT_RUN_AT_BOOT:
    case BVM_BOOT_RUN_IF_NO_HOST:
        return BVM_OK;
    default:
        return BVM_ERR_UNSUPPORTED_VALUE;
    }
}

/* ================================================================== */
/* CRC-16 / COBS                                                      */
/* ================================================================== */

uint16_t bvm_crc16_ccitt_false(const uint8_t *bytes, size_t len) {
    uint16_t crc = 0xFFFFu;
    size_t i;
    for (i = 0; i < len; ++i) {
        int bit;
        crc ^= (uint16_t)((uint16_t)bytes[i] << 8);
        for (bit = 0; bit < 8; ++bit) {
            if ((crc & 0x8000u) != 0) {
                crc = (uint16_t)((uint16_t)(crc << 1) ^ 0x1021u);
            } else {
                crc = (uint16_t)(crc << 1);
            }
        }
    }
    return crc;
}

bvm_error_t bvm_cobs_encode(const uint8_t *input, size_t input_len,
                            uint8_t *out, size_t out_len, size_t *written) {
    size_t read_index = 0;
    size_t write_index = 1;
    size_t code_index = 0;
    uint8_t code = 1;

    if (out_len == 0) {
        return BVM_ERR_OUTPUT_TOO_SMALL;
    }

    while (read_index < input_len) {
        if (input[read_index] == 0) {
            if (code_index >= out_len) {
                return BVM_ERR_OUTPUT_TOO_SMALL;
            }
            out[code_index] = code;
            code_index = write_index;
            if (write_index == (size_t)-1) {
                return BVM_ERR_OUTPUT_TOO_SMALL;
            }
            write_index += 1;
            code = 1;
            read_index += 1;
        } else {
            if (write_index >= out_len) {
                return BVM_ERR_OUTPUT_TOO_SMALL;
            }
            out[write_index] = input[read_index];
            write_index += 1;
            code += 1;
            read_index += 1;

            if (code == 0xFFu) {
                if (code_index >= out_len) {
                    return BVM_ERR_OUTPUT_TOO_SMALL;
                }
                out[code_index] = code;
                if (read_index == input_len) {
                    *written = write_index;
                    return BVM_OK;
                }
                code_index = write_index;
                if (write_index == (size_t)-1) {
                    return BVM_ERR_OUTPUT_TOO_SMALL;
                }
                write_index += 1;
                code = 1;
            }
        }
    }

    if (code_index >= out_len) {
        return BVM_ERR_OUTPUT_TOO_SMALL;
    }
    out[code_index] = code;
    *written = write_index;
    return BVM_OK;
}

bvm_error_t bvm_cobs_decode(const uint8_t *input, size_t input_len,
                            uint8_t *out, size_t out_len, size_t *written) {
    size_t read_index = 0;
    size_t write_index = 0;

    while (read_index < input_len) {
        uint8_t code = input[read_index];
        size_t end;
        size_t copy_len;
        if (code == 0) {
            return BVM_ERR_INVALID_COBS;
        }
        read_index += 1;

        /* end = read_index + (code - 1), overflow-checked. */
        {
            size_t span = (size_t)(code - 1);
            if (span > (size_t)-1 - read_index) {
                return BVM_ERR_INVALID_COBS;
            }
            end = read_index + span;
        }
        if (end > input_len) {
            return BVM_ERR_INVALID_COBS;
        }
        copy_len = end - read_index;
        if (copy_len > out_len - write_index) {
            return BVM_ERR_OUTPUT_TOO_SMALL;
        }
        if (copy_len != 0) {
            memcpy(out + write_index, input + read_index, copy_len);
        }
        write_index += copy_len;
        read_index = end;

        if (code != 0xFFu && read_index < input_len) {
            if (write_index >= out_len) {
                return BVM_ERR_OUTPUT_TOO_SMALL;
            }
            out[write_index] = 0;
            write_index += 1;
        }
    }

    *written = write_index;
    return BVM_OK;
}

/* ================================================================== */
/* Frame / wire-frame codec                                           */
/* ================================================================== */

bvm_error_t bvm_encode_frame(const bvm_frame_t *frame,
                             uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc;
    bvm_error_t err;
    uint16_t crc;

    err = bvm_validate_flags(frame->flags);
    if (err != BVM_OK) {
        return err;
    }
    if (frame->payload_len > (size_t)UINT32_MAX) {
        return BVM_ERR_PAYLOAD_TOO_LARGE;
    }

    enc = bvm_encoder_new(out, out_len);
    err = bvm_encoder_write_u8(&enc, (uint8_t)BVM_PROTOCOL_VERSION);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, frame->flags);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, frame->message_type);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u16(&enc, frame->request_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_uleb128(&enc, (uint32_t)frame->payload_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_slice(&enc, frame->payload, frame->payload_len);
    if (err != BVM_OK) {
        return err;
    }

    crc = bvm_crc16_ccitt_false(enc.out, enc.len);
    err = bvm_encoder_write_u16(&enc, crc);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_frame(const uint8_t *bytes, size_t len, bvm_frame_t *out) {
    size_t crc_offset;
    uint16_t expected_crc;
    uint16_t actual_crc;
    bvm_decoder_t dec;
    bvm_error_t err;
    uint8_t version;
    uint8_t flags;
    uint8_t message_type;
    uint16_t request_id;
    uint32_t payload_len;
    const uint8_t *payload;

    if (len < 8) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    crc_offset = len - BVM_FRAME_CRC_BYTES;
    err = bvm_read_le_u16(bytes, len, crc_offset, &expected_crc);
    if (err != BVM_OK) {
        return err;
    }
    actual_crc = bvm_crc16_ccitt_false(bytes, crc_offset);
    if (expected_crc != actual_crc) {
        return BVM_ERR_BAD_CRC;
    }

    dec = bvm_decoder_new(bytes, crc_offset);
    err = bvm_decoder_read_u8(&dec, &version);
    if (err != BVM_OK) {
        return err;
    }
    if (version != BVM_PROTOCOL_VERSION) {
        return BVM_ERR_UNSUPPORTED_VERSION;
    }
    err = bvm_decoder_read_u8(&dec, &flags);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_validate_flags(flags);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &message_type);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u16(&dec, &request_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_uleb128(&dec, &payload_len);
    if (err != BVM_OK) {
        return err;
    }
    if (bvm_decoder_remaining_len(&dec) != (size_t)payload_len) {
        return BVM_ERR_PAYLOAD_LENGTH_MISMATCH;
    }
    err = bvm_decoder_read_slice(&dec, (size_t)payload_len, &payload);
    if (err != BVM_OK) {
        return err;
    }

    out->flags = flags;
    out->message_type = message_type;
    out->request_id = request_id;
    out->payload = payload;
    out->payload_len = (size_t)payload_len;
    return BVM_OK;
}

bvm_error_t bvm_encode_wire_frame(const uint8_t *raw_with_crc, size_t raw_len,
                                  uint8_t *out, size_t out_len, size_t *written) {
    size_t encoded_len;
    bvm_error_t err = bvm_cobs_encode(raw_with_crc, raw_len, out, out_len, &encoded_len);
    if (err != BVM_OK) {
        return err;
    }
    if (encoded_len >= out_len) {
        return BVM_ERR_OUTPUT_TOO_SMALL;
    }
    out[encoded_len] = 0;
    *written = encoded_len + 1;
    return BVM_OK;
}

bvm_error_t bvm_decode_wire_frame(const uint8_t *wire, size_t wire_len,
                                  uint8_t *out, size_t out_len, size_t *written) {
    if (wire_len == 0) {
        return BVM_ERR_INPUT_TOO_SHORT;
    }
    if (wire[wire_len - 1] != 0) {
        return BVM_ERR_MISSING_TERMINATOR;
    }
    return bvm_cobs_decode(wire, wire_len - 1, out, out_len, written);
}

bvm_error_t bvm_encode_stream_frame(const bvm_frame_t *frame,
                                    uint8_t *raw_out, size_t raw_out_len,
                                    uint8_t *wire_out, size_t wire_out_len,
                                    size_t *written) {
    size_t raw_len;
    bvm_error_t err = bvm_encode_frame(frame, raw_out, raw_out_len, &raw_len);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_encode_wire_frame(raw_out, raw_len, wire_out, wire_out_len, written);
}

bvm_error_t bvm_decode_stream_frame(const uint8_t *wire, size_t wire_len,
                                    uint8_t *raw_out, size_t raw_out_len,
                                    bvm_frame_t *out) {
    size_t raw_len;
    bvm_error_t err = bvm_decode_wire_frame(wire, wire_len, raw_out, raw_out_len, &raw_len);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decode_frame(raw_out, raw_len, out);
}

/* ================================================================== */
/* Per-message payload codecs                                         */
/* ================================================================== */

bvm_error_t bvm_encode_hello(const bvm_hello_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u8(&enc, v->min_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->max_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_string(&enc, v->host_name, v->host_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->host_nonce);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_hello(const uint8_t *bytes, size_t len, bvm_hello_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_u8(&dec, &out->min_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out->max_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_string(&dec, &out->host_name, &out->host_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->host_nonce);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_hello_ack(const bvm_hello_ack_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u8(&enc, v->selected_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_string(&enc, v->board_name, v->board_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_string(&enc, v->runtime_name, v->runtime_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->host_nonce);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->board_nonce);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u16(&enc, v->max_frame_payload);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_hello_ack(const uint8_t *bytes, size_t len, bvm_hello_ack_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_u8(&dec, &out->selected_version);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_string(&dec, &out->board_name, &out->board_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_string(&dec, &out->runtime_name, &out->runtime_name_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->host_nonce);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->board_nonce);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u16(&dec, &out->max_frame_payload);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_capability_descriptor(const bvm_capability_descriptor_t *v,
                                             uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err = bvm_encoder_write_capability_descriptor(&enc, v);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_capability_descriptor(const uint8_t *bytes, size_t len,
                                             bvm_capability_descriptor_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err = bvm_decoder_read_capability_descriptor(&dec, out);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_caps_report(const bvm_caps_report_header_t *header,
                                   const bvm_capability_descriptor_t *caps,
                                   size_t caps_count,
                                   uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc;
    bvm_error_t err;
    size_t i;

    if ((size_t)header->capability_count != caps_count) {
        return BVM_ERR_PAYLOAD_LENGTH_MISMATCH;
    }
    enc = bvm_encoder_new(out, out_len);
    err = bvm_encoder_write_string(&enc, header->board_id, header->board_id_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_string(&enc, header->runtime_id, header->runtime_id_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, header->max_program_bytes);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, header->max_stack_values);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, header->max_handles);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_bool(&enc, header->supports_store_program);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_uleb128(&enc, header->capability_count);
    if (err != BVM_OK) {
        return err;
    }
    for (i = 0; i < caps_count; ++i) {
        err = bvm_encoder_write_capability_descriptor(&enc, &caps[i]);
        if (err != BVM_OK) {
            return err;
        }
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_caps_report_header(const uint8_t *bytes, size_t len,
                                          bvm_caps_report_header_t *out_header,
                                          bvm_decoder_t *out_decoder) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_string(&dec, &out_header->board_id, &out_header->board_id_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_string(&dec, &out_header->runtime_id, &out_header->runtime_id_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out_header->max_program_bytes);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out_header->max_stack_values);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out_header->max_handles);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_bool(&dec, &out_header->supports_store_program);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_uleb128(&dec, &out_header->capability_count);
    if (err != BVM_OK) {
        return err;
    }
    *out_decoder = dec;
    return BVM_OK;
}

bvm_error_t bvm_encode_program_begin(const bvm_program_begin_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->format);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->total_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->program_crc32);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_program_begin(const uint8_t *bytes, size_t len, bvm_program_begin_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    uint8_t format;
    err = bvm_decoder_read_u16(&dec, &out->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &format);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_program_format_validate(format);
    if (err != BVM_OK) {
        return err;
    }
    out->format = format;
    err = bvm_decoder_read_u32(&dec, &out->total_len);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->program_crc32);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_program_chunk(const bvm_program_chunk_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->offset);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_bytes(&enc, v->bytes, v->bytes_len);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_program_chunk(const uint8_t *bytes, size_t len, bvm_program_chunk_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_u16(&dec, &out->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->offset);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_bytes(&dec, &out->bytes, &out->bytes_len);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_program_end(const bvm_program_end_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_program_end(const uint8_t *bytes, size_t len, bvm_program_end_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err = bvm_decoder_read_u16(&dec, &out->program_id);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_run_request(const bvm_run_request_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc;
    bvm_error_t err;
    if ((v->flags & (uint8_t)~BVM_ALLOWED_RUN_FLAGS) != 0) {
        return BVM_ERR_RESERVED_FLAGS;
    }
    enc = bvm_encoder_new(out, out_len);
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->flags);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->instruction_budget);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->time_budget_ms);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_run_request(const uint8_t *bytes, size_t len, bvm_run_request_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    uint16_t program_id;
    uint8_t flags;
    err = bvm_decoder_read_u16(&dec, &program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &flags);
    if (err != BVM_OK) {
        return err;
    }
    if ((flags & (uint8_t)~BVM_ALLOWED_RUN_FLAGS) != 0) {
        return BVM_ERR_RESERVED_FLAGS;
    }
    out->program_id = program_id;
    out->flags = flags;
    err = bvm_decoder_read_u32(&dec, &out->instruction_budget);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->time_budget_ms);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_run_report_header(const bvm_run_report_header_t *v,
                                         uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->status);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->instructions_executed);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->elapsed_ms);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->stack_depth);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->open_handles);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_uleb128(&enc, v->return_count);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_run_report_header(const uint8_t *bytes, size_t len,
                                         bvm_run_report_header_t *out_header,
                                         bvm_decoder_t *out_decoder) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    uint8_t status;
    err = bvm_decoder_read_u16(&dec, &out_header->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &status);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_run_status_validate(status);
    if (err != BVM_OK) {
        return err;
    }
    out_header->status = status;
    err = bvm_decoder_read_u32(&dec, &out_header->instructions_executed);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out_header->elapsed_ms);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out_header->stack_depth);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out_header->open_handles);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_uleb128(&dec, &out_header->return_count);
    if (err != BVM_OK) {
        return err;
    }
    *out_decoder = dec;
    return BVM_OK;
}

bvm_error_t bvm_encode_store_program(const bvm_store_program_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc;
    bvm_error_t err = bvm_validate_boot_policy(v->boot_policy);
    if (err != BVM_OK) {
        return err;
    }
    enc = bvm_encoder_new(out, out_len);
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->slot);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u8(&enc, v->boot_policy);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_store_program(const uint8_t *bytes, size_t len, bvm_store_program_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_u16(&dec, &out->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out->slot);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u8(&dec, &out->boot_policy);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_validate_boot_policy(out->boot_policy);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_error_payload(const bvm_error_payload_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err;
    err = bvm_encoder_write_u16(&enc, v->code);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u16(&enc, v->request_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u16(&enc, v->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_u32(&enc, v->bytecode_offset);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_encoder_write_string(&enc, v->message, v->message_len);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_error_payload(const uint8_t *bytes, size_t len, bvm_error_payload_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err;
    err = bvm_decoder_read_u16(&dec, &out->code);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u16(&dec, &out->request_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u16(&dec, &out->program_id);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_u32(&dec, &out->bytecode_offset);
    if (err != BVM_OK) {
        return err;
    }
    err = bvm_decoder_read_string(&dec, &out->message, &out->message_len);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_ping(const bvm_ping_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err = bvm_encoder_write_u32(&enc, v->nonce);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_ping(const uint8_t *bytes, size_t len, bvm_ping_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err = bvm_decoder_read_u32(&dec, &out->nonce);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_pong(const bvm_pong_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err = bvm_encoder_write_u32(&enc, v->nonce);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_pong(const uint8_t *bytes, size_t len, bvm_pong_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err = bvm_decoder_read_u32(&dec, &out->nonce);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}

bvm_error_t bvm_encode_value(const bvm_value_t *v, uint8_t *out, size_t out_len, size_t *written) {
    bvm_encoder_t enc = bvm_encoder_new(out, out_len);
    bvm_error_t err = bvm_encoder_write_value(&enc, v);
    if (err != BVM_OK) {
        return err;
    }
    *written = enc.len;
    return BVM_OK;
}

bvm_error_t bvm_decode_value(const uint8_t *bytes, size_t len, bvm_value_t *out) {
    bvm_decoder_t dec = bvm_decoder_new(bytes, len);
    bvm_error_t err = bvm_decoder_read_value(&dec, out);
    if (err != BVM_OK) {
        return err;
    }
    return bvm_decoder_finish(&dec);
}
