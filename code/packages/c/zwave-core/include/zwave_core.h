/*
 * zwave_core.h — Z-Wave identifier, region, and Serial API frame primitives.
 * =========================================================================
 *
 * A faithful port of the Rust `zwave-core` crate. It is not a controller: it
 * gives later Z-Wave code a tested byte boundary for controller serial frames,
 * node identity, command-class ids, and regional-profile metadata.
 *
 * The two codecs here parse UNTRUSTED bytes: `zw_serial_frame_parse` (SOF,
 * length, type, function id, payload, XOR checksum) and
 * `zw_command_class_frame_parse` (1- or 2-byte command-class id, command id,
 * payload). Both bounds-check every field and report a structured `ZWaveError`.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef ZWAVE_CORE_H
#define ZWAVE_CORE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Serial control bytes. */
#define ZW_SOF 0x01u
#define ZW_ACK 0x06u
#define ZW_NAK 0x15u
#define ZW_CAN 0x18u

/* Common command-class ids. */
#define ZW_CC_BASIC 0x20u
#define ZW_CC_SWITCH_BINARY 0x25u
#define ZW_CC_SWITCH_MULTILEVEL 0x26u
#define ZW_CC_SENSOR_BINARY 0x30u
#define ZW_CC_SENSOR_MULTILEVEL 0x31u
#define ZW_CC_DOOR_LOCK 0x62u
#define ZW_CC_BATTERY 0x80u
#define ZW_CC_SECURITY_2 0x9fu

/* ── Errors ────────────────────────────────────────────────────────────────*/

typedef enum {
    ZW_OK = 0,
    ZW_ERR_INVALID_CLASSIC_NODE_ID,     /* a = value */
    ZW_ERR_INVALID_LONG_RANGE_NODE_ID,  /* a = value */
    ZW_ERR_MISSING_START_OF_FRAME,      /* a = byte */
    ZW_ERR_INVALID_LENGTH,              /* a = length */
    ZW_ERR_INVALID_FRAME_TYPE,          /* a = byte */
    ZW_ERR_TRUNCATED,                   /* a = needed, b = remaining */
    ZW_ERR_PAYLOAD_TOO_LONG,            /* a = len */
    ZW_ERR_COMMAND_PAYLOAD_TOO_LONG,    /* a = len */
    ZW_ERR_CHECKSUM_MISMATCH,           /* a = expected, b = actual */
    ZW_ERR_OUT_OF_MEMORY
} ZWaveErrorKind;

/* A structured error; `a`/`b` carry the parametric values above. */
typedef struct {
    ZWaveErrorKind kind;
    size_t a;
    size_t b;
} ZWaveError;

const char *zw_error_kind_str(ZWaveErrorKind kind);

/* ── HomeId ────────────────────────────────────────────────────────────────*/

/* Write the 32-bit home id big-endian into `out`. */
void zw_home_id_to_be_bytes(uint32_t home_id, uint8_t out[4]);

/* ── NodeId ────────────────────────────────────────────────────────────────*/

typedef enum { ZW_NODE_CLASSIC, ZW_NODE_LONG_RANGE } ZWaveNodeKind;
typedef struct {
    ZWaveNodeKind kind;
    uint16_t value;
} ZWaveNodeId;

/* Classic node ids are 1..=232; Long Range node ids are 1..=4000. */
ZWaveError zw_node_id_classic(uint8_t value, ZWaveNodeId *out);
ZWaveError zw_node_id_long_range(uint16_t value, ZWaveNodeId *out);
int zw_node_id_is_classic(ZWaveNodeId id);
int zw_node_id_is_long_range(ZWaveNodeId id);

/* ── RegionProfile ─────────────────────────────────────────────────────────*/

typedef enum {
    ZW_REGION_EUROPE,
    ZW_REGION_UNITED_STATES,
    ZW_REGION_AUSTRALIA_NEW_ZEALAND,
    ZW_REGION_HONG_KONG,
    ZW_REGION_INDIA,
    ZW_REGION_ISRAEL,
    ZW_REGION_RUSSIA,
    ZW_REGION_CHINA,
    ZW_REGION_JAPAN,
    ZW_REGION_KOREA,
    ZW_REGION_UNITED_STATES_LONG_RANGE,
    ZW_REGION_EUROPE_LONG_RANGE
} ZWaveRegionProfile;

const char *zw_region_band_description(ZWaveRegionProfile region);
int zw_region_supports_long_range(ZWaveRegionProfile region);

/* ── CommandClassId (a bare u16) ───────────────────────────────────────────*/

/* Command-class ids >= 0x100 are "extended" and encode as two big-endian
 * bytes; smaller ids encode as one byte. */
size_t zw_command_class_encoded_len(uint16_t cc);
/* Encode `cc` into `out` (1 or 2 bytes); returns the number written. */
size_t zw_command_class_encode(uint16_t cc, uint8_t out[2]);
int zw_command_class_is_actuator(uint16_t cc);
int zw_command_class_is_sensor(uint16_t cc);
int zw_command_class_is_security(uint16_t cc);

/* XOR-fold checksum used by the serial framing (seed 0xff). */
uint8_t zw_serial_checksum(const uint8_t *bytes, size_t len);

/* ── ZWaveNetworkSummary ───────────────────────────────────────────────────*/

typedef struct {
    ZWaveRegionProfile region;
    int supports_long_range;
    size_t classic_nodes;
    size_t long_range_nodes;
    size_t command_class_entries;
    size_t actuator_command_classes;
    size_t sensor_command_classes;
    size_t security_command_classes;
} ZWaveNetworkSummary;

ZWaveNetworkSummary zw_network_summary_from_parts(ZWaveRegionProfile region,
                                                  const ZWaveNodeId *nodes,
                                                  size_t n_nodes,
                                                  const uint16_t *ccids,
                                                  size_t n_ccids);
int zw_network_summary_has_nodes(ZWaveNetworkSummary s);
int zw_network_summary_has_long_range_nodes(ZWaveNetworkSummary s);
int zw_network_summary_has_security(ZWaveNetworkSummary s);

/* ── CommandClassFrame ─────────────────────────────────────────────────────*/

typedef struct {
    uint16_t command_class_id;
    uint8_t command_id;
    uint8_t *payload; /* owned */
    size_t payload_len;
} ZWaveCommandClassFrame;

/* Build a frame owning a copy of `payload`. Returns ZW_OK or OOM. */
ZWaveError zw_command_class_frame_init(uint16_t cc, uint8_t command_id,
                                       const uint8_t *payload, size_t len,
                                       ZWaveCommandClassFrame *out);
void zw_command_class_frame_free(ZWaveCommandClassFrame *frame);
/* Parse a command-class frame from bytes (payload is copied into `*out`). */
ZWaveError zw_command_class_frame_parse(const uint8_t *bytes, size_t len,
                                        ZWaveCommandClassFrame *out);
/* Encode a frame into an owned byte buffer (`*out_bytes`/`*out_len`; caller
 * frees). Fails with COMMAND_PAYLOAD_TOO_LONG if the frame exceeds 255 bytes. */
ZWaveError zw_command_class_frame_encode(const ZWaveCommandClassFrame *frame,
                                         uint8_t **out_bytes, size_t *out_len);

typedef struct {
    size_t frame_count;
    size_t short_command_class_frames;
    size_t extended_command_class_frames;
    size_t security_2_frames;
    size_t total_payload_bytes;
    size_t max_payload_bytes;
} ZWaveCommandClassFrameSummary;

ZWaveCommandClassFrameSummary zw_command_class_frame_summary(
    const ZWaveCommandClassFrame *frames, size_t n);
int zw_command_class_frame_summary_has_extended(
    ZWaveCommandClassFrameSummary s);
int zw_command_class_frame_summary_has_security_2(
    ZWaveCommandClassFrameSummary s);
int zw_command_class_frame_summary_is_empty(ZWaveCommandClassFrameSummary s);

/* ── SerialFrame ───────────────────────────────────────────────────────────*/

typedef enum { ZW_SERIAL_REQUEST, ZW_SERIAL_RESPONSE } ZWaveSerialFrameType;

typedef struct {
    ZWaveSerialFrameType frame_type;
    uint8_t function_id;
    uint8_t *payload; /* owned */
    size_t payload_len;
} ZWaveSerialFrame;

ZWaveError zw_serial_frame_init(ZWaveSerialFrameType type, uint8_t function_id,
                                const uint8_t *payload, size_t len,
                                ZWaveSerialFrame *out);
void zw_serial_frame_free(ZWaveSerialFrame *frame);
ZWaveError zw_serial_frame_parse(const uint8_t *bytes, size_t len,
                                 ZWaveSerialFrame *out);
ZWaveError zw_serial_frame_encode(const ZWaveSerialFrame *frame,
                                  uint8_t **out_bytes, size_t *out_len);

typedef struct {
    ZWaveSerialFrameType frame_type;
    uint8_t function_id;
    size_t payload_len;
} ZWaveSerialFrameSummary;

ZWaveSerialFrameSummary zw_serial_frame_summarize(
    const ZWaveSerialFrame *frame);
int zw_serial_frame_summary_is_request(ZWaveSerialFrameSummary s);
int zw_serial_frame_summary_is_response(ZWaveSerialFrameSummary s);
int zw_serial_frame_summary_has_payload(ZWaveSerialFrameSummary s);
int zw_serial_frame_summary_is_function(ZWaveSerialFrameSummary s,
                                        uint8_t function_id);
int zw_serial_frame_summary_is_empty_payload(ZWaveSerialFrameSummary s);

typedef struct {
    size_t frame_count;
    size_t request_frames;
    size_t response_frames;
    size_t total_payload_bytes;
    size_t max_payload_bytes;
} ZWaveSerialFrameBatchSummary;

ZWaveSerialFrameBatchSummary zw_serial_frame_batch_summary(
    const ZWaveSerialFrame *frames, size_t n);
int zw_serial_frame_batch_summary_has_requests(
    ZWaveSerialFrameBatchSummary s);
int zw_serial_frame_batch_summary_has_responses(
    ZWaveSerialFrameBatchSummary s);
int zw_serial_frame_batch_summary_is_empty(ZWaveSerialFrameBatchSummary s);

/* ── ZWaveControllerReadinessSummary ───────────────────────────────────────*/

typedef struct {
    ZWaveNetworkSummary network;
    ZWaveCommandClassFrameSummary command_frames;
    ZWaveSerialFrameBatchSummary serial_frames;
    int has_nodes;
    int has_command_class_coverage;
    int has_serial_requests;
    int has_serial_responses;
    int has_security_coverage;
    int long_range_region_mismatch;
} ZWaveControllerReadinessSummary;

ZWaveControllerReadinessSummary zw_controller_readiness_summary(
    ZWaveNetworkSummary network, ZWaveCommandClassFrameSummary command_frames,
    ZWaveSerialFrameBatchSummary serial_frames);
int zw_controller_readiness_is_ready(ZWaveControllerReadinessSummary s);
int zw_controller_readiness_needs_node_discovery(
    ZWaveControllerReadinessSummary s);
int zw_controller_readiness_needs_command_class_interview(
    ZWaveControllerReadinessSummary s);
int zw_controller_readiness_needs_serial_probe(
    ZWaveControllerReadinessSummary s);
int zw_controller_readiness_waiting_for_serial_response(
    ZWaveControllerReadinessSummary s);
int zw_controller_readiness_needs_region_review(
    ZWaveControllerReadinessSummary s);

#ifdef __cplusplus
}
#endif

#endif /* ZWAVE_CORE_H */
