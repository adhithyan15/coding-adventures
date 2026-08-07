/*
 * board_vm_protocol.h — a host<->board VM wire protocol codec (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * A faithful C port of the Rust `board-vm-protocol` crate.  It defines the
 * framing and message payloads a host uses to talk to a tiny "board VM"
 * (think: a microcontroller running a bytecode interpreter) over a byte
 * stream such as a serial line.
 *
 * The design is deliberately allocation-free: every routine writes into a
 * caller-supplied buffer and returns how many bytes it produced, or decodes
 * a caller-supplied buffer and hands back *borrowed* pointers into it.  This
 * mirrors the Rust crate's `&mut [u8]` / `&[u8]` slice discipline and makes
 * the code usable on a board with no heap at all.
 *
 * Three layers stack on top of each other:
 *
 *   1. Message payloads  — encode_hello / decode_hello, etc.  Each message
 *      type serialises to a compact little-endian payload.
 *   2. Frames            — encode_frame wraps a payload with a version byte,
 *      flags, a message-type tag, a request id, a ULEB128 length, and a
 *      trailing CRC-16/CCITT-FALSE.
 *   3. Wire frames       — encode_wire_frame COBS-encodes a raw frame and
 *      appends a 0x00 terminator so frames can be delimited on a raw stream.
 *
 * Errors.  Rust returns Result<_, ProtocolError>; here every fallible routine
 * returns a bvm_error_t status code (BVM_OK == 0 on success).  The Rust
 * variants that carry a byte (e.g. ReservedFlags) map to a plain status code
 * — the offending byte is not returned, matching the established port
 * convention for data-carrying Rust error enums.
 *
 * Borrowing.  Decoded structs contain pointers into the *input* buffer,
 * together with explicit lengths.  Strings are NOT NUL-terminated; use the
 * paired *_len field.  A decoded struct is only valid while its source buffer
 * lives.
 */
#ifndef BOARD_VM_PROTOCOL_H
#define BOARD_VM_PROTOCOL_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ------------------------------------------------------------------ */
/* Protocol constants                                                 */
/* ------------------------------------------------------------------ */

#define BVM_PROTOCOL_VERSION 1u
#define BVM_FRAME_CRC_BYTES  2u

/* Frame flag bits. */
#define BVM_FLAG_RESPONSE_REQUIRED  0x01u
#define BVM_FLAG_IS_RESPONSE        0x02u
#define BVM_FLAG_IS_ERROR_RESPONSE  0x04u
#define BVM_FLAG_COMPRESSED_PAYLOAD 0x08u
/* Only these flag bits are legal in a v1 frame; anything else is reserved. */
#define BVM_ALLOWED_V1_FLAGS \
    (BVM_FLAG_RESPONSE_REQUIRED | BVM_FLAG_IS_RESPONSE | BVM_FLAG_IS_ERROR_RESPONSE)

/* Well-known capability ids reported by a board. */
#define BVM_CAP_PROGRAM_RAM_EXEC     0x7001u
#define BVM_CAP_PROGRAM_STORE        0x7002u
#define BVM_CAP_TRANSPORT_PIPELINING 0x7003u

/* Capability descriptor flag bits. */
#define BVM_CAP_FLAG_BYTECODE_CALLABLE 0x01u
#define BVM_CAP_FLAG_PROTOCOL_FEATURE  0x02u
#define BVM_CAP_FLAG_BOARD_METADATA    0x04u

/* RUN request flag bits. */
#define BVM_RUN_FLAG_RESET_VM_BEFORE_RUN    0x01u
#define BVM_RUN_FLAG_KEEP_HANDLES_AFTER_RUN 0x02u
#define BVM_RUN_FLAG_BACKGROUND_RUN         0x04u
#define BVM_ALLOWED_RUN_FLAGS \
    (BVM_RUN_FLAG_RESET_VM_BEFORE_RUN | BVM_RUN_FLAG_KEEP_HANDLES_AFTER_RUN | \
     BVM_RUN_FLAG_BACKGROUND_RUN)

/* Boot policy for a stored program. */
#define BVM_BOOT_STORE_ONLY     0x00u
#define BVM_BOOT_RUN_AT_BOOT    0x01u
#define BVM_BOOT_RUN_IF_NO_HOST 0x02u

/* Sentinels. */
#define BVM_NO_PROGRAM_ID      0xFFFFu
#define BVM_NO_BYTECODE_OFFSET 0xFFFFFFFFu

/* ------------------------------------------------------------------ */
/* Message type tags                                                  */
/* ------------------------------------------------------------------ */

#define BVM_MSG_HELLO             0x01u
#define BVM_MSG_HELLO_ACK         0x02u
#define BVM_MSG_CAPS_QUERY        0x03u
#define BVM_MSG_CAPS_REPORT       0x04u
#define BVM_MSG_PROGRAM_BEGIN     0x05u
#define BVM_MSG_PROGRAM_CHUNK     0x06u
#define BVM_MSG_PROGRAM_END       0x07u
#define BVM_MSG_RUN               0x08u
#define BVM_MSG_RUN_REPORT        0x09u
#define BVM_MSG_STOP              0x0Au
#define BVM_MSG_RESET_VM          0x0Bu
#define BVM_MSG_STORE_PROGRAM     0x0Cu
#define BVM_MSG_RUN_STORED        0x0Du
#define BVM_MSG_READ_STATE        0x0Eu
#define BVM_MSG_STATE_REPORT      0x0Fu
#define BVM_MSG_SUBSCRIBE         0x10u
#define BVM_MSG_EVENT             0x11u
#define BVM_MSG_LOG               0x12u
#define BVM_MSG_ERROR             0x13u
#define BVM_MSG_PING              0x14u
#define BVM_MSG_PONG              0x15u
#define BVM_MSG_BOOTLOADER_REBOOT 0x16u

/* Message type tags >= 0x80 are reserved for vendor extensions. */
int bvm_message_type_is_vendor_extension(uint8_t message_type);

/* ------------------------------------------------------------------ */
/* Program format / run status wire values                            */
/* ------------------------------------------------------------------ */

/* The only defined program format is a BVM module (wire byte 0x01). */
#define BVM_PROGRAM_FORMAT_BVM_MODULE 0x01u

/* Run status wire values. */
#define BVM_RUN_STATUS_HALTED          0x00u
#define BVM_RUN_STATUS_RUNNING         0x01u
#define BVM_RUN_STATUS_STOPPED         0x02u
#define BVM_RUN_STATUS_BUDGET_EXCEEDED 0x03u
#define BVM_RUN_STATUS_FAULTED         0x04u

/* ------------------------------------------------------------------ */
/* Error codes                                                        */
/* ------------------------------------------------------------------ */

typedef enum bvm_error {
    BVM_OK = 0,                       /* success sentinel (not in Rust) */
    BVM_ERR_OUTPUT_TOO_SMALL,         /* ProtocolError::OutputTooSmall */
    BVM_ERR_INPUT_TOO_SHORT,          /* ProtocolError::InputTooShort */
    BVM_ERR_MISSING_TERMINATOR,       /* ProtocolError::MissingTerminator */
    BVM_ERR_INVALID_COBS,             /* ProtocolError::InvalidCobs */
    BVM_ERR_TRUNCATED_ULEB,           /* ProtocolError::TruncatedUleb */
    BVM_ERR_ULEB_OVERFLOW,            /* ProtocolError::UlebOverflow */
    BVM_ERR_PAYLOAD_TOO_LARGE,        /* ProtocolError::PayloadTooLarge */
    BVM_ERR_PAYLOAD_LENGTH_MISMATCH,  /* ProtocolError::PayloadLengthMismatch */
    BVM_ERR_BAD_CRC,                  /* ProtocolError::BadCrc */
    BVM_ERR_UNSUPPORTED_VERSION,      /* ProtocolError::UnsupportedVersion */
    BVM_ERR_RESERVED_FLAGS,           /* ProtocolError::ReservedFlags */
    BVM_ERR_UNSUPPORTED_VALUE,        /* ProtocolError::UnsupportedValue */
    BVM_ERR_INVALID_BOOL,             /* ProtocolError::InvalidBool */
    BVM_ERR_INVALID_UTF8,             /* ProtocolError::InvalidUtf8 */
    BVM_ERR_TRAILING_BYTES            /* ProtocolError::TrailingBytes */
} bvm_error_t;

/* ------------------------------------------------------------------ */
/* Golden test vectors (exported so tests / users can pin the wire)   */
/* ------------------------------------------------------------------ */

extern const uint8_t BVM_GOLDEN_HELLO_PAYLOAD_BVM_V1[10];
extern const uint8_t BVM_GOLDEN_HELLO_RAW_FRAME_BVM_V1[18];
extern const uint8_t BVM_GOLDEN_HELLO_WIRE_FRAME_BVM_V1[20];
extern const uint8_t BVM_GOLDEN_PROGRAM_BEGIN_PAYLOAD_BVM_V1[11];
extern const uint8_t BVM_GOLDEN_RUN_BACKGROUND_PAYLOAD_BVM_V1[11];

/* ------------------------------------------------------------------ */
/* Message structs                                                    */
/* ------------------------------------------------------------------ */

/*
 * Decoded strings/byte-slices point INTO the source buffer and are not
 * NUL-terminated; always read exactly the paired length.  When encoding,
 * supply the same pointer + length; a length of 0 is legal (pointer may be
 * NULL).
 */

typedef struct {
    uint8_t flags;
    uint8_t message_type;
    uint16_t request_id;
    const uint8_t *payload;
    size_t payload_len;
} bvm_frame_t;

typedef struct {
    uint8_t min_version;
    uint8_t max_version;
    const char *host_name;
    size_t host_name_len;
    uint32_t host_nonce;
} bvm_hello_t;

typedef struct {
    uint8_t selected_version;
    const char *board_name;
    size_t board_name_len;
    const char *runtime_name;
    size_t runtime_name_len;
    uint32_t host_nonce;
    uint32_t board_nonce;
    uint16_t max_frame_payload;
} bvm_hello_ack_t;

typedef struct {
    uint16_t id;
    uint8_t version;
    uint16_t flags;
    const char *name;
    size_t name_len;
} bvm_capability_descriptor_t;

typedef struct {
    const char *board_id;
    size_t board_id_len;
    const char *runtime_id;
    size_t runtime_id_len;
    uint32_t max_program_bytes;
    uint8_t max_stack_values;
    uint8_t max_handles;
    int supports_store_program; /* boolean */
    uint32_t capability_count;
} bvm_caps_report_header_t;

typedef struct {
    uint16_t program_id;
    uint8_t format; /* BVM_PROGRAM_FORMAT_* */
    uint32_t total_len;
    uint32_t program_crc32;
} bvm_program_begin_t;

typedef struct {
    uint16_t program_id;
    uint32_t offset;
    const uint8_t *bytes;
    size_t bytes_len;
} bvm_program_chunk_t;

typedef struct {
    uint16_t program_id;
} bvm_program_end_t;

typedef struct {
    uint16_t program_id;
    uint8_t flags;
    uint32_t instruction_budget;
    uint32_t time_budget_ms;
} bvm_run_request_t;

typedef struct {
    uint16_t program_id;
    uint8_t status; /* BVM_RUN_STATUS_* */
    uint32_t instructions_executed;
    uint32_t elapsed_ms;
    uint8_t stack_depth;
    uint8_t open_handles;
    uint32_t return_count;
} bvm_run_report_header_t;

typedef struct {
    uint16_t program_id;
    uint8_t slot;
    uint8_t boot_policy; /* BVM_BOOT_* */
} bvm_store_program_t;

typedef struct {
    uint16_t code;
    uint16_t request_id;
    uint16_t program_id;
    uint32_t bytecode_offset;
    const char *message;
    size_t message_len;
} bvm_error_payload_t;

typedef struct {
    uint32_t nonce;
} bvm_ping_t;

typedef struct {
    uint32_t nonce;
} bvm_pong_t;

/* Tagged VM value. */
typedef enum {
    BVM_VALUE_UNIT   = 0x00,
    BVM_VALUE_BOOL   = 0x01,
    BVM_VALUE_U8     = 0x02,
    BVM_VALUE_U16    = 0x03,
    BVM_VALUE_U32    = 0x04,
    BVM_VALUE_I16    = 0x05,
    BVM_VALUE_HANDLE = 0x06,
    BVM_VALUE_BYTES  = 0x07,
    BVM_VALUE_STRING = 0x08
} bvm_value_tag_t;

typedef struct {
    bvm_value_tag_t tag;
    union {
        int boolean;      /* BVM_VALUE_BOOL */
        uint8_t u8;       /* BVM_VALUE_U8 */
        uint16_t u16;     /* BVM_VALUE_U16 */
        uint32_t u32;     /* BVM_VALUE_U32 */
        int16_t i16;      /* BVM_VALUE_I16 */
        uint16_t handle;  /* BVM_VALUE_HANDLE */
        struct { const uint8_t *ptr; size_t len; } bytes;  /* BVM_VALUE_BYTES */
        struct { const char *ptr; size_t len; } str;       /* BVM_VALUE_STRING */
    } as;
} bvm_value_t;

/* ------------------------------------------------------------------ */
/* Encoder / Decoder (public, for advanced / streaming use)           */
/* ------------------------------------------------------------------ */

typedef struct bvm_encoder {
    uint8_t *out;
    size_t cap;
    size_t len;
} bvm_encoder_t;

typedef struct bvm_decoder {
    const uint8_t *input;
    size_t input_len;
    size_t offset;
} bvm_decoder_t;

/* ------------------------------------------------------------------ */
/* Low-level primitives                                               */
/* ------------------------------------------------------------------ */

/* CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF, no reflection, no xorout). */
uint16_t bvm_crc16_ccitt_false(const uint8_t *bytes, size_t len);

/* COBS: consistent-overhead byte stuffing.  Both write into `out`. */
bvm_error_t bvm_cobs_encode(const uint8_t *input, size_t input_len,
                            uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_cobs_decode(const uint8_t *input, size_t input_len,
                            uint8_t *out, size_t out_len, size_t *written);

/* Validate program-format / run-status wire bytes (BVM_OK iff recognised). */
bvm_error_t bvm_program_format_validate(uint8_t value);
bvm_error_t bvm_run_status_validate(uint8_t value);

/* ------------------------------------------------------------------ */
/* Frame / wire-frame codec                                           */
/* ------------------------------------------------------------------ */

bvm_error_t bvm_encode_frame(const bvm_frame_t *frame,
                             uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_frame(const uint8_t *bytes, size_t len, bvm_frame_t *out);

bvm_error_t bvm_encode_wire_frame(const uint8_t *raw_with_crc, size_t raw_len,
                                  uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_wire_frame(const uint8_t *wire, size_t wire_len,
                                  uint8_t *out, size_t out_len, size_t *written);

/* Compose frame + wire-frame.  `raw_out` is scratch for the raw frame. */
bvm_error_t bvm_encode_stream_frame(const bvm_frame_t *frame,
                                    uint8_t *raw_out, size_t raw_out_len,
                                    uint8_t *wire_out, size_t wire_out_len,
                                    size_t *written);
bvm_error_t bvm_decode_stream_frame(const uint8_t *wire, size_t wire_len,
                                    uint8_t *raw_out, size_t raw_out_len,
                                    bvm_frame_t *out);

/* ------------------------------------------------------------------ */
/* Per-message payload codecs                                         */
/* ------------------------------------------------------------------ */

bvm_error_t bvm_encode_hello(const bvm_hello_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_hello(const uint8_t *bytes, size_t len, bvm_hello_t *out);

bvm_error_t bvm_encode_hello_ack(const bvm_hello_ack_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_hello_ack(const uint8_t *bytes, size_t len, bvm_hello_ack_t *out);

bvm_error_t bvm_encode_capability_descriptor(const bvm_capability_descriptor_t *v,
                                             uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_capability_descriptor(const uint8_t *bytes, size_t len,
                                             bvm_capability_descriptor_t *out);

/* CapsReport: a header followed by `capability_count` descriptors.  The
 * decode hands back a decoder positioned at the first descriptor so the
 * caller can iterate with bvm_decoder_read_capability_descriptor. */
bvm_error_t bvm_encode_caps_report(const bvm_caps_report_header_t *header,
                                   const bvm_capability_descriptor_t *caps,
                                   size_t caps_count,
                                   uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_caps_report_header(const uint8_t *bytes, size_t len,
                                          bvm_caps_report_header_t *out_header,
                                          bvm_decoder_t *out_decoder);

bvm_error_t bvm_encode_program_begin(const bvm_program_begin_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_program_begin(const uint8_t *bytes, size_t len, bvm_program_begin_t *out);

bvm_error_t bvm_encode_program_chunk(const bvm_program_chunk_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_program_chunk(const uint8_t *bytes, size_t len, bvm_program_chunk_t *out);

bvm_error_t bvm_encode_program_end(const bvm_program_end_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_program_end(const uint8_t *bytes, size_t len, bvm_program_end_t *out);

bvm_error_t bvm_encode_run_request(const bvm_run_request_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_run_request(const uint8_t *bytes, size_t len, bvm_run_request_t *out);

bvm_error_t bvm_encode_run_report_header(const bvm_run_report_header_t *v,
                                         uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_run_report_header(const uint8_t *bytes, size_t len,
                                         bvm_run_report_header_t *out_header,
                                         bvm_decoder_t *out_decoder);

bvm_error_t bvm_encode_store_program(const bvm_store_program_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_store_program(const uint8_t *bytes, size_t len, bvm_store_program_t *out);

bvm_error_t bvm_encode_error_payload(const bvm_error_payload_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_error_payload(const uint8_t *bytes, size_t len, bvm_error_payload_t *out);

bvm_error_t bvm_encode_ping(const bvm_ping_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_ping(const uint8_t *bytes, size_t len, bvm_ping_t *out);

bvm_error_t bvm_encode_pong(const bvm_pong_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_pong(const uint8_t *bytes, size_t len, bvm_pong_t *out);

bvm_error_t bvm_encode_value(const bvm_value_t *v, uint8_t *out, size_t out_len, size_t *written);
bvm_error_t bvm_decode_value(const uint8_t *bytes, size_t len, bvm_value_t *out);

/* ------------------------------------------------------------------ */
/* Encoder / Decoder methods                                          */
/* ------------------------------------------------------------------ */

bvm_encoder_t bvm_encoder_new(uint8_t *out, size_t cap);
size_t bvm_encoder_len(const bvm_encoder_t *enc);
int bvm_encoder_is_empty(const bvm_encoder_t *enc);

bvm_error_t bvm_encoder_write_slice(bvm_encoder_t *enc, const uint8_t *value, size_t len);
bvm_error_t bvm_encoder_write_u8(bvm_encoder_t *enc, uint8_t value);
bvm_error_t bvm_encoder_write_bool(bvm_encoder_t *enc, int value);
bvm_error_t bvm_encoder_write_u16(bvm_encoder_t *enc, uint16_t value);
bvm_error_t bvm_encoder_write_u32(bvm_encoder_t *enc, uint32_t value);
bvm_error_t bvm_encoder_write_i16(bvm_encoder_t *enc, int16_t value);
bvm_error_t bvm_encoder_write_uleb128(bvm_encoder_t *enc, uint32_t value);
bvm_error_t bvm_encoder_write_string(bvm_encoder_t *enc, const char *value, size_t len);
bvm_error_t bvm_encoder_write_bytes(bvm_encoder_t *enc, const uint8_t *value, size_t len);
bvm_error_t bvm_encoder_write_capability_descriptor(bvm_encoder_t *enc,
                                                    const bvm_capability_descriptor_t *v);
bvm_error_t bvm_encoder_write_value(bvm_encoder_t *enc, const bvm_value_t *v);

bvm_decoder_t bvm_decoder_new(const uint8_t *input, size_t len);
size_t bvm_decoder_offset(const bvm_decoder_t *dec);
size_t bvm_decoder_remaining_len(const bvm_decoder_t *dec);
bvm_error_t bvm_decoder_finish(const bvm_decoder_t *dec);

bvm_error_t bvm_decoder_read_u8(bvm_decoder_t *dec, uint8_t *out);
bvm_error_t bvm_decoder_read_bool(bvm_decoder_t *dec, int *out);
bvm_error_t bvm_decoder_read_u16(bvm_decoder_t *dec, uint16_t *out);
bvm_error_t bvm_decoder_read_u32(bvm_decoder_t *dec, uint32_t *out);
bvm_error_t bvm_decoder_read_i16(bvm_decoder_t *dec, int16_t *out);
bvm_error_t bvm_decoder_read_uleb128(bvm_decoder_t *dec, uint32_t *out);
/* read_string/read_bytes/read_slice yield a borrowed pointer + length. */
bvm_error_t bvm_decoder_read_string(bvm_decoder_t *dec, const char **out, size_t *out_len);
bvm_error_t bvm_decoder_read_bytes(bvm_decoder_t *dec, const uint8_t **out, size_t *out_len);
bvm_error_t bvm_decoder_read_slice(bvm_decoder_t *dec, size_t len, const uint8_t **out);
bvm_error_t bvm_decoder_read_capability_descriptor(bvm_decoder_t *dec,
                                                   bvm_capability_descriptor_t *out);
bvm_error_t bvm_decoder_read_value(bvm_decoder_t *dec, bvm_value_t *out);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* BOARD_VM_PROTOCOL_H */
