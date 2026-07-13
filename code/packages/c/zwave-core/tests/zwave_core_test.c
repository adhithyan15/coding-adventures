/* Tests for zwave-core, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests. */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "zwave_core.h"

int main(void) {
    /* ── node id ranges ────────────────────────────────────────────────────*/
    {
        ZWaveNodeId id;
        ISO_CHECK(zw_node_id_classic(1, &id).kind == ZW_OK &&
                  id.kind == ZW_NODE_CLASSIC && id.value == 1);
        ISO_CHECK(zw_node_id_classic(0, &id).kind ==
                  ZW_ERR_INVALID_CLASSIC_NODE_ID);
        ISO_CHECK(zw_node_id_classic(233, &id).kind ==
                  ZW_ERR_INVALID_CLASSIC_NODE_ID);
        ISO_CHECK(zw_node_id_long_range(4000, &id).kind == ZW_OK &&
                  id.kind == ZW_NODE_LONG_RANGE && id.value == 4000);
        {
            ZWaveError e = zw_node_id_long_range(4001, &id);
            ISO_CHECK(e.kind == ZW_ERR_INVALID_LONG_RANGE_NODE_ID &&
                      e.a == 4001);
        }
        ISO_CHECK(zw_node_id_long_range(0, &id).kind ==
                  ZW_ERR_INVALID_LONG_RANGE_NODE_ID);
    }

    /* ── home id big-endian ────────────────────────────────────────────────*/
    {
        uint8_t out[4];
        static const uint8_t expect[4] = {0xDE, 0xAD, 0xBE, 0xEF};
        zw_home_id_to_be_bytes(0xDEADBEEFu, out);
        ISO_CHECK_MEM_EQ(out, expect, 4);
    }

    /* ── region profiles ───────────────────────────────────────────────────*/
    {
        ISO_CHECK(!zw_region_supports_long_range(ZW_REGION_UNITED_STATES));
        ISO_CHECK(
            zw_region_supports_long_range(ZW_REGION_UNITED_STATES_LONG_RANGE));
        ISO_CHECK_STR_EQ(zw_region_band_description(ZW_REGION_EUROPE_LONG_RANGE),
                         "EU Z-Wave Long Range");
        ISO_CHECK_STR_EQ(zw_region_band_description(ZW_REGION_UNITED_STATES),
                         "US sub-GHz");
    }

    /* ── command-class ids and classification ──────────────────────────────*/
    {
        ISO_CHECK_EQ_UINT(ZW_CC_SWITCH_BINARY, 0x25u);
        ISO_CHECK_EQ_UINT(ZW_CC_BATTERY, 0x80u);
        ISO_CHECK_EQ_UINT(ZW_CC_SECURITY_2, 0x9fu);
        ISO_CHECK_EQ_UINT(zw_command_class_encoded_len(0x25), 1u);
        ISO_CHECK_EQ_UINT(zw_command_class_encoded_len(0xf100), 2u);
        ISO_CHECK(zw_command_class_is_actuator(ZW_CC_SWITCH_BINARY));
        ISO_CHECK(!zw_command_class_is_actuator(ZW_CC_SENSOR_BINARY));
        ISO_CHECK(zw_command_class_is_sensor(ZW_CC_SENSOR_MULTILEVEL));
        ISO_CHECK(zw_command_class_is_security(ZW_CC_SECURITY_2));
    }

    /* ── network summary ───────────────────────────────────────────────────*/
    {
        ZWaveNodeId nodes[2];
        uint16_t ccs[3] = {ZW_CC_SWITCH_BINARY, ZW_CC_SENSOR_MULTILEVEL,
                           ZW_CC_SECURITY_2};
        ZWaveNetworkSummary s;
        (void)zw_node_id_classic(2, &nodes[0]);
        (void)zw_node_id_long_range(2001, &nodes[1]);
        s = zw_network_summary_from_parts(ZW_REGION_UNITED_STATES_LONG_RANGE,
                                          nodes, 2, ccs, 3);
        ISO_CHECK(s.supports_long_range && s.classic_nodes == 1 &&
                  s.long_range_nodes == 1);
        ISO_CHECK(s.command_class_entries == 3 &&
                  s.actuator_command_classes == 1 &&
                  s.sensor_command_classes == 1 &&
                  s.security_command_classes == 1);
        ISO_CHECK(zw_network_summary_has_nodes(s) &&
                  zw_network_summary_has_long_range_nodes(s) &&
                  zw_network_summary_has_security(s));
        {
            ZWaveNetworkSummary empty = zw_network_summary_from_parts(
                ZW_REGION_EUROPE, NULL, 0, NULL, 0);
            ISO_CHECK(!empty.supports_long_range &&
                      !zw_network_summary_has_nodes(empty) &&
                      !zw_network_summary_has_security(empty));
        }
    }

    /* ── command-class frame round-trips ───────────────────────────────────*/
    {
        ZWaveCommandClassFrame f, p;
        uint8_t *enc;
        size_t enc_len;
        static const uint8_t payload[1] = {0xff};
        static const uint8_t expect[3] = {0x25, 0x01, 0xff};
        ISO_CHECK(zw_command_class_frame_init(ZW_CC_SWITCH_BINARY, 0x01,
                                              payload, 1, &f)
                      .kind == ZW_OK);
        ISO_CHECK(zw_command_class_frame_encode(&f, &enc, &enc_len).kind ==
                  ZW_OK);
        ISO_CHECK(enc_len == 3);
        ISO_CHECK_MEM_EQ(enc, expect, 3);
        ISO_CHECK(zw_command_class_frame_parse(enc, enc_len, &p).kind == ZW_OK);
        ISO_CHECK(p.command_class_id == ZW_CC_SWITCH_BINARY &&
                  p.command_id == 0x01 && p.payload_len == 1 &&
                  p.payload[0] == 0xff);
        free(enc);
        zw_command_class_frame_free(&f);
        zw_command_class_frame_free(&p);
    }
    {
        /* extended id 0xf100 */
        ZWaveCommandClassFrame f, p;
        uint8_t *enc;
        size_t enc_len;
        static const uint8_t payload[2] = {0x01, 0x02};
        static const uint8_t expect[5] = {0xf1, 0x00, 0x02, 0x01, 0x02};
        ISO_CHECK(zw_command_class_frame_init(0xf100, 0x02, payload, 2, &f)
                      .kind == ZW_OK);
        ISO_CHECK(zw_command_class_frame_encode(&f, &enc, &enc_len).kind ==
                  ZW_OK);
        ISO_CHECK(enc_len == 5);
        ISO_CHECK_MEM_EQ(enc, expect, 5);
        ISO_CHECK(zw_command_class_frame_parse(enc, enc_len, &p).kind == ZW_OK);
        ISO_CHECK(p.command_class_id == 0xf100 && p.command_id == 0x02 &&
                  p.payload_len == 2);
        free(enc);
        zw_command_class_frame_free(&f);
        zw_command_class_frame_free(&p);
    }
    {
        /* truncated command-class frames */
        ZWaveCommandClassFrame p;
        static const uint8_t a[1] = {0xf1};
        static const uint8_t b[1] = {0x25};
        ZWaveError e = zw_command_class_frame_parse(a, 1, &p);
        ISO_CHECK(e.kind == ZW_ERR_TRUNCATED && e.a == 2 && e.b == 1);
        e = zw_command_class_frame_parse(b, 1, &p);
        ISO_CHECK(e.kind == ZW_ERR_TRUNCATED && e.a == 2 && e.b == 1);
    }

    /* ── command-class frame summary ───────────────────────────────────────*/
    {
        ZWaveCommandClassFrame frames[3];
        ZWaveCommandClassFrameSummary s;
        static const uint8_t p1[1] = {0xff};
        static const uint8_t p2[2] = {0x01, 0x02};
        static const uint8_t p3[1] = {0x55};
        (void)zw_command_class_frame_init(ZW_CC_BASIC, 0x01, p1, 1, &frames[0]);
        (void)zw_command_class_frame_init(ZW_CC_SECURITY_2, 0x02, p2, 2,
                                          &frames[1]);
        (void)zw_command_class_frame_init(0xf100, 0x03, p3, 1, &frames[2]);
        s = zw_command_class_frame_summary(frames, 3);
        ISO_CHECK(s.frame_count == 3 && s.short_command_class_frames == 2 &&
                  s.extended_command_class_frames == 1 &&
                  s.security_2_frames == 1 && s.total_payload_bytes == 4 &&
                  s.max_payload_bytes == 2);
        ISO_CHECK(zw_command_class_frame_summary_has_extended(s) &&
                  zw_command_class_frame_summary_has_security_2(s) &&
                  !zw_command_class_frame_summary_is_empty(s));
        zw_command_class_frame_free(&frames[0]);
        zw_command_class_frame_free(&frames[1]);
        zw_command_class_frame_free(&frames[2]);
    }

    /* ── serial frame round-trip + checksum ────────────────────────────────*/
    {
        ZWaveSerialFrame f, p;
        uint8_t *enc;
        size_t enc_len;
        static const uint8_t payload[3] = {0x02, 0x25, 0x01};
        ZWaveSerialFrameSummary sum;
        ISO_CHECK(zw_serial_frame_init(ZW_SERIAL_REQUEST, 0x13, payload, 3, &f)
                      .kind == ZW_OK);
        ISO_CHECK(zw_serial_frame_encode(&f, &enc, &enc_len).kind == ZW_OK);
        ISO_CHECK(enc[0] == ZW_SOF);
        ISO_CHECK(zw_serial_frame_parse(enc, enc_len, &p).kind == ZW_OK);
        ISO_CHECK(p.frame_type == ZW_SERIAL_REQUEST && p.function_id == 0x13 &&
                  p.payload_len == 3 && memcmp(p.payload, payload, 3) == 0);
        sum = zw_serial_frame_summarize(&f);
        ISO_CHECK(sum.frame_type == ZW_SERIAL_REQUEST &&
                  sum.function_id == 0x13 && sum.payload_len == 3);
        ISO_CHECK(zw_serial_frame_summary_is_request(sum) &&
                  !zw_serial_frame_summary_is_response(sum) &&
                  zw_serial_frame_summary_has_payload(sum) &&
                  zw_serial_frame_summary_is_function(sum, 0x13) &&
                  !zw_serial_frame_summary_is_function(sum, 0x02));
        free(enc);
        zw_serial_frame_free(&f);
        zw_serial_frame_free(&p);
    }

    /* ── checksum mismatch rejected ────────────────────────────────────────*/
    {
        ZWaveSerialFrame f, p;
        uint8_t *enc;
        size_t enc_len;
        static const uint8_t payload[2] = {0x01, 0x02};
        (void)zw_serial_frame_init(ZW_SERIAL_RESPONSE, 0x02, payload, 2, &f);
        (void)zw_serial_frame_encode(&f, &enc, &enc_len);
        enc[enc_len - 1] ^= 0x01;
        ISO_CHECK(zw_serial_frame_parse(enc, enc_len, &p).kind ==
                  ZW_ERR_CHECKSUM_MISMATCH);
        free(enc);
        zw_serial_frame_free(&f);
    }
    /* missing SOF and truncation */
    {
        ZWaveSerialFrame p;
        static const uint8_t no_sof[5] = {0x02, 0x03, 0x00, 0x13, 0x00};
        static const uint8_t tiny[3] = {0x01, 0x03, 0x00};
        ISO_CHECK(zw_serial_frame_parse(no_sof, 5, &p).kind ==
                  ZW_ERR_MISSING_START_OF_FRAME);
        ISO_CHECK(zw_serial_frame_parse(tiny, 3, &p).kind == ZW_ERR_TRUNCATED);
    }

    /* ── serial frame batch summary ────────────────────────────────────────*/
    {
        ZWaveSerialFrame frames[2];
        ZWaveSerialFrameBatchSummary s;
        static const uint8_t p1[3] = {0x02, 0x25, 0x01};
        static const uint8_t p2[1] = {0x01};
        (void)zw_serial_frame_init(ZW_SERIAL_REQUEST, 0x13, p1, 3, &frames[0]);
        (void)zw_serial_frame_init(ZW_SERIAL_RESPONSE, 0x02, p2, 1, &frames[1]);
        s = zw_serial_frame_batch_summary(frames, 2);
        ISO_CHECK(s.frame_count == 2 && s.request_frames == 1 &&
                  s.response_frames == 1 && s.total_payload_bytes == 4 &&
                  s.max_payload_bytes == 3);
        ISO_CHECK(zw_serial_frame_batch_summary_has_requests(s) &&
                  zw_serial_frame_batch_summary_has_responses(s) &&
                  !zw_serial_frame_batch_summary_is_empty(s));
        zw_serial_frame_free(&frames[0]);
        zw_serial_frame_free(&frames[1]);
    }

    /* ── controller readiness: ready ───────────────────────────────────────*/
    {
        ZWaveNodeId nodes[2];
        uint16_t ccs[1] = {ZW_CC_SWITCH_BINARY};
        ZWaveCommandClassFrame cframe;
        ZWaveSerialFrame both[2];
        ZWaveNetworkSummary net;
        ZWaveCommandClassFrameSummary cfs;
        ZWaveSerialFrameBatchSummary sfs;
        ZWaveControllerReadinessSummary r;
        static const uint8_t cp[1] = {0x01};
        static const uint8_t rp[3] = {0x02, 0x25, 0x01};
        static const uint8_t pp[1] = {0x01};
        (void)zw_node_id_classic(2, &nodes[0]);
        (void)zw_node_id_long_range(2001, &nodes[1]);
        net = zw_network_summary_from_parts(ZW_REGION_UNITED_STATES_LONG_RANGE,
                                            nodes, 2, ccs, 1);
        (void)zw_command_class_frame_init(ZW_CC_SECURITY_2, 0x02, cp, 1,
                                          &cframe);
        cfs = zw_command_class_frame_summary(&cframe, 1);
        (void)zw_serial_frame_init(ZW_SERIAL_REQUEST, 0x13, rp, 3, &both[0]);
        (void)zw_serial_frame_init(ZW_SERIAL_RESPONSE, 0x02, pp, 1, &both[1]);
        sfs = zw_serial_frame_batch_summary(both, 2);
        r = zw_controller_readiness_summary(net, cfs, sfs);
        ISO_CHECK(zw_controller_readiness_is_ready(r));
        ISO_CHECK(!zw_controller_readiness_needs_node_discovery(r) &&
                  !zw_controller_readiness_needs_command_class_interview(r) &&
                  !zw_controller_readiness_needs_serial_probe(r) &&
                  !zw_controller_readiness_waiting_for_serial_response(r) &&
                  !zw_controller_readiness_needs_region_review(r));
        zw_command_class_frame_free(&cframe);
        zw_serial_frame_free(&both[0]);
        zw_serial_frame_free(&both[1]);
    }
    /* ── controller readiness: blocked ─────────────────────────────────────*/
    {
        ZWaveNodeId nodes[1];
        ZWaveSerialFrame sreq;
        ZWaveNetworkSummary net;
        ZWaveCommandClassFrameSummary cfs;
        ZWaveSerialFrameBatchSummary sfs;
        ZWaveControllerReadinessSummary r;
        static const uint8_t rp[3] = {0x02, 0x25, 0x01};
        (void)zw_node_id_long_range(2001, &nodes[0]);
        net = zw_network_summary_from_parts(ZW_REGION_UNITED_STATES, nodes, 1,
                                            NULL, 0);
        cfs = zw_command_class_frame_summary(NULL, 0);
        (void)zw_serial_frame_init(ZW_SERIAL_REQUEST, 0x13, rp, 3, &sreq);
        sfs = zw_serial_frame_batch_summary(&sreq, 1);
        r = zw_controller_readiness_summary(net, cfs, sfs);
        ISO_CHECK(!zw_controller_readiness_is_ready(r));
        ISO_CHECK(zw_controller_readiness_needs_command_class_interview(r) &&
                  zw_controller_readiness_waiting_for_serial_response(r) &&
                  zw_controller_readiness_needs_region_review(r) &&
                  !r.has_security_coverage);
        zw_serial_frame_free(&sreq);
    }

    /* ── checksum helper ───────────────────────────────────────────────────*/
    {
        static const uint8_t data[3] = {0x03, 0x00, 0x13};
        ISO_CHECK_EQ_UINT(zw_serial_checksum(data, 3),
                          (unsigned)(0xff ^ 0x03 ^ 0x00 ^ 0x13));
    }

    return ISO_TEST_RESULT();
}
