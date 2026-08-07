/*
 * zwave_core.c — implementation of the pure-ISO C Z-Wave core primitives.
 * =====================================================================
 *
 * See zwave_core.h. Frames own a malloc'd payload copy; the two parsers
 * bounds-check untrusted input and never read past the given length.
 */
#include "zwave_core.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy */

/* ── Errors ────────────────────────────────────────────────────────────────*/

static ZWaveError zw_ok(void) {
    ZWaveError e;
    e.kind = ZW_OK;
    e.a = 0;
    e.b = 0;
    return e;
}
static ZWaveError zw_err1(ZWaveErrorKind kind, size_t a) {
    ZWaveError e;
    e.kind = kind;
    e.a = a;
    e.b = 0;
    return e;
}
static ZWaveError zw_err2(ZWaveErrorKind kind, size_t a, size_t b) {
    ZWaveError e;
    e.kind = kind;
    e.a = a;
    e.b = b;
    return e;
}

const char *zw_error_kind_str(ZWaveErrorKind kind) {
    switch (kind) {
        case ZW_OK: return "ok";
        case ZW_ERR_INVALID_CLASSIC_NODE_ID: return "invalid classic node id";
        case ZW_ERR_INVALID_LONG_RANGE_NODE_ID:
            return "invalid long range node id";
        case ZW_ERR_MISSING_START_OF_FRAME: return "missing start of frame";
        case ZW_ERR_INVALID_LENGTH: return "invalid frame length";
        case ZW_ERR_INVALID_FRAME_TYPE: return "invalid frame type";
        case ZW_ERR_TRUNCATED: return "truncated frame";
        case ZW_ERR_PAYLOAD_TOO_LONG: return "serial payload too long";
        case ZW_ERR_COMMAND_PAYLOAD_TOO_LONG:
            return "command-class payload too long";
        case ZW_ERR_CHECKSUM_MISMATCH: return "checksum mismatch";
        case ZW_ERR_OUT_OF_MEMORY: return "out of memory";
    }
    return "unknown error";
}

/* Copy `len` bytes from `src` into a fresh buffer (NULL for len==0). Returns 1
 * on success, 0 on OOM. */
static int dup_bytes(const uint8_t *src, size_t len, uint8_t **out) {
    if (len == 0) {
        *out = NULL;
        return 1;
    }
    *out = (uint8_t *)malloc(len);
    if (*out == NULL) return 0;
    memcpy(*out, src, len);
    return 1;
}

/* ── HomeId ────────────────────────────────────────────────────────────────*/

void zw_home_id_to_be_bytes(uint32_t home_id, uint8_t out[4]) {
    out[0] = (uint8_t)((home_id >> 24) & 0xff);
    out[1] = (uint8_t)((home_id >> 16) & 0xff);
    out[2] = (uint8_t)((home_id >> 8) & 0xff);
    out[3] = (uint8_t)(home_id & 0xff);
}

/* ── NodeId ────────────────────────────────────────────────────────────────*/

ZWaveError zw_node_id_classic(uint8_t value, ZWaveNodeId *out) {
    if (value == 0 || value > 232)
        return zw_err1(ZW_ERR_INVALID_CLASSIC_NODE_ID, value);
    out->kind = ZW_NODE_CLASSIC;
    out->value = value;
    return zw_ok();
}
ZWaveError zw_node_id_long_range(uint16_t value, ZWaveNodeId *out) {
    if (value < 1 || value > 4000)
        return zw_err1(ZW_ERR_INVALID_LONG_RANGE_NODE_ID, value);
    out->kind = ZW_NODE_LONG_RANGE;
    out->value = value;
    return zw_ok();
}
int zw_node_id_is_classic(ZWaveNodeId id) { return id.kind == ZW_NODE_CLASSIC; }
int zw_node_id_is_long_range(ZWaveNodeId id) {
    return id.kind == ZW_NODE_LONG_RANGE;
}

/* ── RegionProfile ─────────────────────────────────────────────────────────*/

const char *zw_region_band_description(ZWaveRegionProfile region) {
    switch (region) {
        case ZW_REGION_EUROPE: return "EU sub-GHz";
        case ZW_REGION_UNITED_STATES: return "US sub-GHz";
        case ZW_REGION_AUSTRALIA_NEW_ZEALAND: return "ANZ sub-GHz";
        case ZW_REGION_HONG_KONG: return "Hong Kong sub-GHz";
        case ZW_REGION_INDIA: return "India sub-GHz";
        case ZW_REGION_ISRAEL: return "Israel sub-GHz";
        case ZW_REGION_RUSSIA: return "Russia sub-GHz";
        case ZW_REGION_CHINA: return "China sub-GHz";
        case ZW_REGION_JAPAN: return "Japan sub-GHz";
        case ZW_REGION_KOREA: return "Korea sub-GHz";
        case ZW_REGION_UNITED_STATES_LONG_RANGE: return "US Z-Wave Long Range";
        case ZW_REGION_EUROPE_LONG_RANGE: return "EU Z-Wave Long Range";
    }
    return "unknown region";
}
int zw_region_supports_long_range(ZWaveRegionProfile region) {
    return region == ZW_REGION_UNITED_STATES_LONG_RANGE ||
           region == ZW_REGION_EUROPE_LONG_RANGE;
}

/* ── CommandClassId ────────────────────────────────────────────────────────*/

size_t zw_command_class_encoded_len(uint16_t cc) { return cc <= 0xff ? 1 : 2; }
size_t zw_command_class_encode(uint16_t cc, uint8_t out[2]) {
    if (cc <= 0xff) {
        out[0] = (uint8_t)cc;
        return 1;
    }
    out[0] = (uint8_t)((cc >> 8) & 0xff);
    out[1] = (uint8_t)(cc & 0xff);
    return 2;
}
int zw_command_class_is_actuator(uint16_t cc) {
    return cc == ZW_CC_BASIC || cc == ZW_CC_SWITCH_BINARY ||
           cc == ZW_CC_SWITCH_MULTILEVEL || cc == ZW_CC_DOOR_LOCK;
}
int zw_command_class_is_sensor(uint16_t cc) {
    return cc == ZW_CC_SENSOR_BINARY || cc == ZW_CC_SENSOR_MULTILEVEL;
}
int zw_command_class_is_security(uint16_t cc) { return cc == ZW_CC_SECURITY_2; }

uint8_t zw_serial_checksum(const uint8_t *bytes, size_t len) {
    uint8_t acc = 0xff;
    size_t i;
    for (i = 0; i < len; i++) acc ^= bytes[i];
    return acc;
}

/* ── ZWaveNetworkSummary ───────────────────────────────────────────────────*/

ZWaveNetworkSummary zw_network_summary_from_parts(ZWaveRegionProfile region,
                                                  const ZWaveNodeId *nodes,
                                                  size_t n_nodes,
                                                  const uint16_t *ccids,
                                                  size_t n_ccids) {
    ZWaveNetworkSummary s;
    size_t i;
    s.region = region;
    s.supports_long_range = zw_region_supports_long_range(region);
    s.classic_nodes = 0;
    s.long_range_nodes = 0;
    s.command_class_entries = 0;
    s.actuator_command_classes = 0;
    s.sensor_command_classes = 0;
    s.security_command_classes = 0;
    for (i = 0; i < n_nodes; i++) {
        if (zw_node_id_is_long_range(nodes[i]))
            s.long_range_nodes++;
        else
            s.classic_nodes++;
    }
    for (i = 0; i < n_ccids; i++) {
        s.command_class_entries++;
        if (zw_command_class_is_actuator(ccids[i])) s.actuator_command_classes++;
        if (zw_command_class_is_sensor(ccids[i])) s.sensor_command_classes++;
        if (zw_command_class_is_security(ccids[i]))
            s.security_command_classes++;
    }
    return s;
}
int zw_network_summary_has_nodes(ZWaveNetworkSummary s) {
    return s.classic_nodes + s.long_range_nodes > 0;
}
int zw_network_summary_has_long_range_nodes(ZWaveNetworkSummary s) {
    return s.long_range_nodes > 0;
}
int zw_network_summary_has_security(ZWaveNetworkSummary s) {
    return s.security_command_classes > 0;
}

/* ── CommandClassFrame ─────────────────────────────────────────────────────*/

ZWaveError zw_command_class_frame_init(uint16_t cc, uint8_t command_id,
                                       const uint8_t *payload, size_t len,
                                       ZWaveCommandClassFrame *out) {
    out->command_class_id = cc;
    out->command_id = command_id;
    out->payload_len = len;
    if (!dup_bytes(payload, len, &out->payload))
        return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    return zw_ok();
}
void zw_command_class_frame_free(ZWaveCommandClassFrame *frame) {
    if (frame == NULL) return;
    free(frame->payload);
    frame->payload = NULL;
    frame->payload_len = 0;
}

/* Parse a command-class id: ids >= 0xf0 in the first byte take a second byte. */
static ZWaveError parse_command_class_id(const uint8_t *bytes, size_t len,
                                         uint16_t *out_cc,
                                         size_t *out_next_off) {
    uint8_t first;
    if (len < 1) return zw_err2(ZW_ERR_TRUNCATED, 1, 0);
    first = bytes[0];
    if (first >= 0xf0) {
        if (len < 2) return zw_err2(ZW_ERR_TRUNCATED, 2, len);
        *out_cc = (uint16_t)(((uint16_t)first << 8) | bytes[1]);
        *out_next_off = 2;
    } else {
        *out_cc = (uint16_t)first;
        *out_next_off = 1;
    }
    return zw_ok();
}

ZWaveError zw_command_class_frame_parse(const uint8_t *bytes, size_t len,
                                        ZWaveCommandClassFrame *out) {
    uint16_t cc;
    size_t cmd_off;
    size_t payload_len;
    ZWaveError e = parse_command_class_id(bytes, len, &cc, &cmd_off);
    if (e.kind != ZW_OK) return e;
    if (cmd_off >= len) return zw_err2(ZW_ERR_TRUNCATED, cmd_off + 1, len);
    out->command_class_id = cc;
    out->command_id = bytes[cmd_off];
    payload_len = len - (cmd_off + 1);
    out->payload_len = payload_len;
    if (!dup_bytes(bytes + cmd_off + 1, payload_len, &out->payload))
        return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    return zw_ok();
}

ZWaveError zw_command_class_frame_encode(const ZWaveCommandClassFrame *frame,
                                         uint8_t **out_bytes, size_t *out_len) {
    size_t cc_len = zw_command_class_encoded_len(frame->command_class_id);
    size_t total;
    uint8_t *out;
    uint8_t cc_bytes[2];
    *out_bytes = NULL;
    *out_len = 0;
    /* cc_len (1..2) + 1 (command id) + payload; reject if it overflows 255. */
    if (frame->payload_len > (size_t)-1 - cc_len - 1)
        return zw_err1(ZW_ERR_COMMAND_PAYLOAD_TOO_LONG, frame->payload_len);
    total = cc_len + 1 + frame->payload_len;
    if (total > 255)
        return zw_err1(ZW_ERR_COMMAND_PAYLOAD_TOO_LONG, frame->payload_len);
    out = (uint8_t *)malloc(total);
    if (out == NULL) return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    (void)zw_command_class_encode(frame->command_class_id, cc_bytes);
    memcpy(out, cc_bytes, cc_len);
    out[cc_len] = frame->command_id;
    if (frame->payload_len > 0)
        memcpy(out + cc_len + 1, frame->payload, frame->payload_len);
    *out_bytes = out;
    *out_len = total;
    return zw_ok();
}

ZWaveCommandClassFrameSummary zw_command_class_frame_summary(
    const ZWaveCommandClassFrame *frames, size_t n) {
    ZWaveCommandClassFrameSummary s;
    size_t i;
    s.frame_count = 0;
    s.short_command_class_frames = 0;
    s.extended_command_class_frames = 0;
    s.security_2_frames = 0;
    s.total_payload_bytes = 0;
    s.max_payload_bytes = 0;
    for (i = 0; i < n; i++) {
        s.frame_count++;
        if (zw_command_class_encoded_len(frames[i].command_class_id) == 1)
            s.short_command_class_frames++;
        else
            s.extended_command_class_frames++;
        if (frames[i].command_class_id == ZW_CC_SECURITY_2)
            s.security_2_frames++;
        s.total_payload_bytes += frames[i].payload_len;
        if (frames[i].payload_len > s.max_payload_bytes)
            s.max_payload_bytes = frames[i].payload_len;
    }
    return s;
}
int zw_command_class_frame_summary_has_extended(
    ZWaveCommandClassFrameSummary s) {
    return s.extended_command_class_frames > 0;
}
int zw_command_class_frame_summary_has_security_2(
    ZWaveCommandClassFrameSummary s) {
    return s.security_2_frames > 0;
}
int zw_command_class_frame_summary_is_empty(ZWaveCommandClassFrameSummary s) {
    return s.frame_count == 0;
}

/* ── SerialFrame ───────────────────────────────────────────────────────────*/

ZWaveError zw_serial_frame_init(ZWaveSerialFrameType type, uint8_t function_id,
                                const uint8_t *payload, size_t len,
                                ZWaveSerialFrame *out) {
    out->frame_type = type;
    out->function_id = function_id;
    out->payload_len = len;
    if (!dup_bytes(payload, len, &out->payload))
        return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    return zw_ok();
}
void zw_serial_frame_free(ZWaveSerialFrame *frame) {
    if (frame == NULL) return;
    free(frame->payload);
    frame->payload = NULL;
    frame->payload_len = 0;
}

static ZWaveError serial_frame_type_from_byte(uint8_t byte,
                                              ZWaveSerialFrameType *out) {
    if (byte == 0x00) {
        *out = ZW_SERIAL_REQUEST;
        return zw_ok();
    }
    if (byte == 0x01) {
        *out = ZW_SERIAL_RESPONSE;
        return zw_ok();
    }
    return zw_err1(ZW_ERR_INVALID_FRAME_TYPE, byte);
}
static uint8_t serial_frame_type_as_byte(ZWaveSerialFrameType t) {
    return t == ZW_SERIAL_REQUEST ? 0x00 : 0x01;
}

ZWaveError zw_serial_frame_parse(const uint8_t *bytes, size_t len,
                                 ZWaveSerialFrame *out) {
    size_t declared, frame_len;
    uint8_t checksum, expected;
    ZWaveSerialFrameType type;
    ZWaveError e;
    size_t payload_len;

    if (len < 5) return zw_err2(ZW_ERR_TRUNCATED, 5, len);
    if (bytes[0] != ZW_SOF)
        return zw_err1(ZW_ERR_MISSING_START_OF_FRAME, bytes[0]);
    declared = bytes[1];
    if (declared < 3) return zw_err1(ZW_ERR_INVALID_LENGTH, declared);
    frame_len = declared + 2;
    if (len < frame_len) return zw_err2(ZW_ERR_TRUNCATED, frame_len, len);

    checksum = bytes[frame_len - 1];
    expected = zw_serial_checksum(bytes + 1, frame_len - 1 - 1); /* [1, end-1) */
    if (checksum != expected)
        return zw_err2(ZW_ERR_CHECKSUM_MISMATCH, expected, checksum);

    e = serial_frame_type_from_byte(bytes[2], &type);
    if (e.kind != ZW_OK) return e;

    out->frame_type = type;
    out->function_id = bytes[3];
    payload_len = (frame_len - 1) - 4; /* bytes[4 .. frame_len-1) */
    out->payload_len = payload_len;
    if (!dup_bytes(bytes + 4, payload_len, &out->payload))
        return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    return zw_ok();
}

ZWaveError zw_serial_frame_encode(const ZWaveSerialFrame *frame,
                                  uint8_t **out_bytes, size_t *out_len) {
    size_t declared, total;
    uint8_t *out;
    *out_bytes = NULL;
    *out_len = 0;
    /* declared length = payload + 3 (type, function id, checksum); <= 255. */
    if (frame->payload_len > (size_t)-1 - 3)
        return zw_err1(ZW_ERR_PAYLOAD_TOO_LONG, frame->payload_len);
    declared = frame->payload_len + 3;
    if (declared > 255)
        return zw_err1(ZW_ERR_PAYLOAD_TOO_LONG, frame->payload_len);
    total = declared + 2; /* SOF + length byte + declared body */
    out = (uint8_t *)malloc(total);
    if (out == NULL) return zw_err1(ZW_ERR_OUT_OF_MEMORY, 0);
    out[0] = ZW_SOF;
    out[1] = (uint8_t)declared;
    out[2] = serial_frame_type_as_byte(frame->frame_type);
    out[3] = frame->function_id;
    if (frame->payload_len > 0)
        memcpy(out + 4, frame->payload, frame->payload_len);
    out[total - 1] = zw_serial_checksum(out + 1, total - 1 - 1);
    *out_bytes = out;
    *out_len = total;
    return zw_ok();
}

ZWaveSerialFrameSummary zw_serial_frame_summarize(
    const ZWaveSerialFrame *frame) {
    ZWaveSerialFrameSummary s;
    s.frame_type = frame->frame_type;
    s.function_id = frame->function_id;
    s.payload_len = frame->payload_len;
    return s;
}
int zw_serial_frame_summary_is_request(ZWaveSerialFrameSummary s) {
    return s.frame_type == ZW_SERIAL_REQUEST;
}
int zw_serial_frame_summary_is_response(ZWaveSerialFrameSummary s) {
    return s.frame_type == ZW_SERIAL_RESPONSE;
}
int zw_serial_frame_summary_has_payload(ZWaveSerialFrameSummary s) {
    return s.payload_len > 0;
}
int zw_serial_frame_summary_is_function(ZWaveSerialFrameSummary s,
                                        uint8_t function_id) {
    return s.function_id == function_id;
}
int zw_serial_frame_summary_is_empty_payload(ZWaveSerialFrameSummary s) {
    return s.payload_len == 0;
}

ZWaveSerialFrameBatchSummary zw_serial_frame_batch_summary(
    const ZWaveSerialFrame *frames, size_t n) {
    ZWaveSerialFrameBatchSummary s;
    size_t i;
    s.frame_count = 0;
    s.request_frames = 0;
    s.response_frames = 0;
    s.total_payload_bytes = 0;
    s.max_payload_bytes = 0;
    for (i = 0; i < n; i++) {
        s.frame_count++;
        if (frames[i].frame_type == ZW_SERIAL_REQUEST)
            s.request_frames++;
        else
            s.response_frames++;
        s.total_payload_bytes += frames[i].payload_len;
        if (frames[i].payload_len > s.max_payload_bytes)
            s.max_payload_bytes = frames[i].payload_len;
    }
    return s;
}
int zw_serial_frame_batch_summary_has_requests(ZWaveSerialFrameBatchSummary s) {
    return s.request_frames > 0;
}
int zw_serial_frame_batch_summary_has_responses(ZWaveSerialFrameBatchSummary s) {
    return s.response_frames > 0;
}
int zw_serial_frame_batch_summary_is_empty(ZWaveSerialFrameBatchSummary s) {
    return s.frame_count == 0;
}

/* ── ZWaveControllerReadinessSummary ───────────────────────────────────────*/

ZWaveControllerReadinessSummary zw_controller_readiness_summary(
    ZWaveNetworkSummary network, ZWaveCommandClassFrameSummary command_frames,
    ZWaveSerialFrameBatchSummary serial_frames) {
    ZWaveControllerReadinessSummary s;
    s.network = network;
    s.command_frames = command_frames;
    s.serial_frames = serial_frames;
    s.has_nodes = zw_network_summary_has_nodes(network);
    s.has_command_class_coverage =
        network.command_class_entries > 0 ||
        !zw_command_class_frame_summary_is_empty(command_frames);
    s.has_serial_requests = zw_serial_frame_batch_summary_has_requests(
        serial_frames);
    s.has_serial_responses = zw_serial_frame_batch_summary_has_responses(
        serial_frames);
    s.has_security_coverage =
        zw_network_summary_has_security(network) ||
        zw_command_class_frame_summary_has_security_2(command_frames);
    s.long_range_region_mismatch =
        zw_network_summary_has_long_range_nodes(network) &&
        !network.supports_long_range;
    return s;
}
int zw_controller_readiness_is_ready(ZWaveControllerReadinessSummary s) {
    return s.has_nodes && s.has_command_class_coverage &&
           s.has_serial_requests && s.has_serial_responses &&
           !s.long_range_region_mismatch;
}
int zw_controller_readiness_needs_node_discovery(
    ZWaveControllerReadinessSummary s) {
    return !s.has_nodes;
}
int zw_controller_readiness_needs_command_class_interview(
    ZWaveControllerReadinessSummary s) {
    return s.has_nodes && !s.has_command_class_coverage;
}
int zw_controller_readiness_needs_serial_probe(
    ZWaveControllerReadinessSummary s) {
    return !s.has_serial_requests;
}
int zw_controller_readiness_waiting_for_serial_response(
    ZWaveControllerReadinessSummary s) {
    return s.has_serial_requests && !s.has_serial_responses;
}
int zw_controller_readiness_needs_region_review(
    ZWaveControllerReadinessSummary s) {
    return s.long_range_region_mismatch;
}
