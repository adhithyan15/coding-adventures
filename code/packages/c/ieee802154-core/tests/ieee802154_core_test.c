/*
 * Tests for ieee802154-core, mirroring the Rust crate's unit tests (frame
 * parse/encode, auxiliary security header, beacon payload, PAN descriptor and
 * scan summary), using the header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "ieee802154_core.h"

#include <stdlib.h>
#include <string.h>

static IE_FrameControl data_frame_control(void) {
    IE_FrameControl fc;
    memset(&fc, 0, sizeof fc);
    fc.frame_type = IE_FRAME_DATA;
    fc.pan_id_compression = 1;
    fc.destination_address_mode = IE_ADDR_SHORT;
    fc.frame_version = IE_VERSION_2006;
    fc.source_address_mode = IE_ADDR_SHORT;
    return fc;
}

static int fc_eq(IE_FrameControl a, IE_FrameControl b) {
    return a.frame_type == b.frame_type &&
           a.security_enabled == b.security_enabled &&
           a.frame_pending == b.frame_pending &&
           a.ack_request == b.ack_request &&
           a.pan_id_compression == b.pan_id_compression &&
           a.sequence_number_suppression == b.sequence_number_suppression &&
           a.information_elements_present == b.information_elements_present &&
           a.destination_address_mode == b.destination_address_mode &&
           a.frame_version == b.frame_version &&
           a.source_address_mode == b.source_address_mode;
}

int main(void) {
    /* ── parse short-address data frame without FCS ───────────────────────── */
    {
        uint8_t bytes[] = {0x41, 0x98, 0x07, 0x34, 0x12, 0x78,
                           0x56, 0xbc, 0x9a, 0x01, 0x02};
        IE_MacFrame f;
        ISO_CHECK_EQ_INT(ie_mac_frame_parse(bytes, sizeof bytes, 0, &f),
                         IE_MAC_OK);
        ISO_CHECK(fc_eq(f.frame_control, data_frame_control()));
        ISO_CHECK(f.has_sequence_number && f.sequence_number == 7);
        ISO_CHECK(f.has_destination_pan_id && f.destination_pan_id == 0x1234);
        ISO_CHECK(f.has_destination && f.destination.mode == IE_ADDR_SHORT &&
                  f.destination.short_addr == 0x5678);
        ISO_CHECK(f.has_source_pan_id && f.source_pan_id == 0x1234);
        ISO_CHECK(f.has_source && f.source.short_addr == 0x9abc);
        ISO_CHECK(!f.has_aux_security_header);
        ISO_CHECK_EQ_UINT(f.payload_len, 2);
        {
            uint8_t want[] = {0x01, 0x02};
            ISO_CHECK_MEM_EQ(f.payload, want, 2);
        }
        ISO_CHECK(!f.has_fcs);
        ie_mac_frame_free(&f);
    }

    /* ── encode short-address data frame without FCS ──────────────────────── */
    {
        IE_MacFrame f;
        uint8_t *out = NULL;
        size_t out_len = 0;
        uint8_t want[] = {0x41, 0x98, 0x07, 0x34, 0x12, 0x78,
                          0x56, 0xbc, 0x9a, 0x01, 0x02};
        uint8_t payload[] = {0x01, 0x02};
        memset(&f, 0, sizeof f);
        f.frame_control = data_frame_control();
        f.has_sequence_number = 1;
        f.sequence_number = 7;
        f.has_destination_pan_id = 1;
        f.destination_pan_id = 0x1234;
        f.has_destination = 1;
        f.destination.mode = IE_ADDR_SHORT;
        f.destination.short_addr = 0x5678;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        f.has_source = 1;
        f.source.mode = IE_ADDR_SHORT;
        f.source.short_addr = 0x9abc;
        f.payload = payload;
        f.payload_len = 2;
        ISO_CHECK_EQ_INT(ie_mac_frame_encode(&f, &out, &out_len), IE_MAC_OK);
        ISO_CHECK_EQ_UINT(out_len, sizeof want);
        ISO_CHECK_MEM_EQ(out, want, sizeof want);
        free(out);
    }

    /* ── ack frame ────────────────────────────────────────────────────────── */
    {
        uint8_t bytes[] = {0x02, 0x00, 0x2a};
        IE_MacFrame f;
        ISO_CHECK_EQ_INT(ie_mac_frame_parse(bytes, sizeof bytes, 0, &f),
                         IE_MAC_OK);
        ISO_CHECK(f.frame_control.frame_type == IE_FRAME_ACK);
        ISO_CHECK(f.has_sequence_number && f.sequence_number == 0x2a);
        ISO_CHECK(!f.has_destination && !f.has_source);
        ISO_CHECK_EQ_UINT(f.payload_len, 0);
        ie_mac_frame_free(&f);
    }

    /* ── frame with FCS ───────────────────────────────────────────────────── */
    {
        uint8_t bytes[] = {0x02, 0x00, 0x2a, 0xef, 0xbe};
        IE_MacFrame f;
        ISO_CHECK_EQ_INT(ie_mac_frame_parse(bytes, sizeof bytes, 1, &f),
                         IE_MAC_OK);
        ISO_CHECK(f.sequence_number == 0x2a);
        ISO_CHECK(f.has_fcs && f.fcs == 0xbeef);
        ISO_CHECK_EQ_UINT(f.payload_len, 0);
        ie_mac_frame_free(&f);
    }

    /* ── summary of ack frame ─────────────────────────────────────────────── */
    {
        uint8_t bytes[] = {0x02, 0x00, 0x2a};
        IE_MacFrame f;
        IE_MacFrameSummary s;
        ie_mac_frame_parse(bytes, sizeof bytes, 0, &f);
        ie_mac_frame_summary(&f, &s);
        ISO_CHECK(s.frame_type == IE_FRAME_ACK);
        ISO_CHECK(!ie_mac_summary_has_addressing(&s));
        ISO_CHECK(!ie_mac_summary_has_payload(&s));
        ISO_CHECK(!s.security_enabled);
        ISO_CHECK(!s.has_auxiliary_security_header);
        ISO_CHECK(s.has_sequence_number);
        ISO_CHECK(!s.has_fcs);
        ie_mac_frame_free(&f);
    }

    /* ── summary of full data frame ───────────────────────────────────────── */
    {
        IE_MacFrame f;
        IE_MacFrameSummary s;
        uint8_t payload[] = {0xaa, 0xbb, 0xcc};
        memset(&f, 0, sizeof f);
        f.frame_control = data_frame_control();
        f.frame_control.ack_request = 1;
        f.frame_control.frame_pending = 1;
        f.has_sequence_number = 1;
        f.sequence_number = 7;
        f.has_destination_pan_id = 1;
        f.destination_pan_id = 0x1234;
        f.has_destination = 1;
        f.destination.mode = IE_ADDR_SHORT;
        f.destination.short_addr = 0x5678;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        f.has_source = 1;
        f.source.mode = IE_ADDR_SHORT;
        f.source.short_addr = 0x9abc;
        f.payload = payload;
        f.payload_len = 3;
        f.has_fcs = 1;
        f.fcs = 0xbeef;
        ie_mac_frame_summary(&f, &s);
        ISO_CHECK(s.frame_type == IE_FRAME_DATA);
        ISO_CHECK(s.frame_version == IE_VERSION_2006);
        ISO_CHECK(s.destination_address_mode == IE_ADDR_SHORT);
        ISO_CHECK(s.ack_request && s.frame_pending && s.pan_id_compression);
        ISO_CHECK(!s.sequence_number_suppressed && s.has_sequence_number);
        ISO_CHECK(s.has_destination_pan_id && s.has_source_pan_id);
        ISO_CHECK(s.has_destination && s.has_source);
        ISO_CHECK(ie_mac_summary_has_addressing(&s));
        ISO_CHECK_EQ_UINT(s.payload_len, 3);
        ISO_CHECK(ie_mac_summary_has_payload(&s) && s.has_fcs);
    }

    /* ── sequence number suppression ──────────────────────────────────────── */
    {
        uint8_t bytes[] = {0x41, 0x99, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a};
        IE_MacFrame f;
        ie_mac_frame_parse(bytes, sizeof bytes, 0, &f);
        ISO_CHECK(f.frame_control.sequence_number_suppression);
        ISO_CHECK(!f.has_sequence_number);
        ISO_CHECK_EQ_UINT(f.payload_len, 0);
        ie_mac_frame_free(&f);
    }

    /* ── reserved address mode is rejected ────────────────────────────────── */
    {
        uint8_t bytes[] = {0x01, 0x04, 0x07};
        IE_MacFrame f;
        ISO_CHECK_EQ_INT(ie_mac_frame_parse(bytes, sizeof bytes, 0, &f),
                         IE_MAC_ERR_RESERVED_ADDRESS_MODE);
    }

    /* ── aux security header with key index ───────────────────────────────── */
    {
        uint8_t bytes[] = {0x49, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56,
                           0xbc, 0x9a, 0x0d, 0x44, 0x33, 0x22, 0x11,
                           0x02, 0xaa, 0xbb};
        IE_MacFrame f;
        ie_mac_frame_parse(bytes, sizeof bytes, 0, &f);
        ISO_CHECK(f.has_aux_security_header);
        ISO_CHECK(f.aux_security_header.security_control.security_level ==
                  IE_SEC_ENC_MIC32);
        ISO_CHECK(f.aux_security_header.security_control.key_identifier_mode ==
                  IE_KEYID_KEY_INDEX);
        ISO_CHECK(f.aux_security_header.has_frame_counter &&
                  !f.aux_security_header.frame_counter.is_40bit &&
                  f.aux_security_header.frame_counter.value == 0x11223344u);
        ISO_CHECK(f.aux_security_header.key_identifier.mode ==
                  IE_KEYID_KEY_INDEX);
        ISO_CHECK(f.aux_security_header.key_identifier.index == 2);
        {
            uint8_t want[] = {0xaa, 0xbb};
            ISO_CHECK_MEM_EQ(f.payload, want, 2);
        }
        ie_mac_frame_free(&f);
    }

    /* ── encode aux security header with key source 8 ─────────────────────── */
    {
        IE_MacFrame f;
        uint8_t *out = NULL;
        size_t out_len = 0;
        uint8_t payload[] = {0xaa};
        uint8_t want[] = {0x49, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a,
                          0x5e, 0x05, 0x04, 0x03, 0x02, 0x01, 0x10, 0x11, 0x12,
                          0x13, 0x14, 0x15, 0x16, 0x17, 0x22, 0xaa};
        memset(&f, 0, sizeof f);
        f.frame_control = data_frame_control();
        f.frame_control.security_enabled = 1;
        f.has_sequence_number = 1;
        f.sequence_number = 7;
        f.has_destination_pan_id = 1;
        f.destination_pan_id = 0x1234;
        f.has_destination = 1;
        f.destination.mode = IE_ADDR_SHORT;
        f.destination.short_addr = 0x5678;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        f.has_source = 1;
        f.source.mode = IE_ADDR_SHORT;
        f.source.short_addr = 0x9abc;
        f.has_aux_security_header = 1;
        f.aux_security_header.security_control.security_level = IE_SEC_ENC_MIC64;
        f.aux_security_header.security_control.key_identifier_mode =
            IE_KEYID_KEY_SOURCE8;
        f.aux_security_header.security_control.frame_counter_suppression = 0;
        f.aux_security_header.security_control.frame_counter_size_5 = 1;
        f.aux_security_header.has_frame_counter = 1;
        f.aux_security_header.frame_counter.is_40bit = 1;
        f.aux_security_header.frame_counter.value = 0x0001020304050ull >> 0;
        f.aux_security_header.frame_counter.value = 0x000102030405ull;
        f.aux_security_header.key_identifier.mode = IE_KEYID_KEY_SOURCE8;
        {
            uint8_t src[8] = {0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17};
            memcpy(f.aux_security_header.key_identifier.source, src, 8);
        }
        f.aux_security_header.key_identifier.index = 0x22;
        f.payload = payload;
        f.payload_len = 1;
        ISO_CHECK_EQ_INT(ie_mac_frame_encode(&f, &out, &out_len), IE_MAC_OK);
        ISO_CHECK_EQ_UINT(out_len, sizeof want);
        ISO_CHECK_MEM_EQ(out, want, sizeof want);
        free(out);
    }

    /* ── encode with security enabled but no aux header ───────────────────── */
    {
        IE_MacFrame f;
        uint8_t *out = NULL;
        size_t out_len = 0;
        memset(&f, 0, sizeof f);
        f.frame_control = data_frame_control();
        f.frame_control.security_enabled = 1;
        f.has_sequence_number = 1;
        f.sequence_number = 7;
        f.has_destination_pan_id = 1;
        f.destination_pan_id = 0x1234;
        f.has_destination = 1;
        f.destination.mode = IE_ADDR_SHORT;
        f.destination.short_addr = 0x5678;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        f.has_source = 1;
        f.source.mode = IE_ADDR_SHORT;
        f.source.short_addr = 0x9abc;
        ISO_CHECK_EQ_INT(ie_mac_frame_encode(&f, &out, &out_len),
                         IE_MAC_ERR_MISSING_AUX_SECURITY_HEADER);
    }

    /* ── security level helpers ───────────────────────────────────────────── */
    ISO_CHECK(!ie_security_level_encrypts(IE_SEC_MIC64));
    ISO_CHECK_EQ_UINT(ie_security_level_mic_len(IE_SEC_MIC64), 8);
    ISO_CHECK(ie_security_level_encrypts(IE_SEC_ENC_MIC128));
    ISO_CHECK_EQ_UINT(ie_security_level_mic_len(IE_SEC_ENC_MIC128), 16);

    /* ── beacon payload with pending addresses ────────────────────────────── */
    {
        uint8_t bytes[] = {0xff, 0xdf, 0x80, 0x11, 0x34, 0x12, 0x11, 0x22,
                           0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0xaa, 0xbb};
        IE_BeaconPayload bp;
        ISO_CHECK_EQ_INT(ie_beacon_payload_parse(bytes, sizeof bytes, &bp),
                         IE_BEACON_OK);
        ISO_CHECK_EQ_UINT(bp.superframe_raw, 0xdfff);
        ISO_CHECK_EQ_INT(ie_superframe_beacon_order(bp.superframe_raw), 15);
        ISO_CHECK_EQ_INT(ie_superframe_order(bp.superframe_raw), 15);
        ISO_CHECK_EQ_INT(ie_superframe_final_cap_slot(bp.superframe_raw), 15);
        ISO_CHECK(ie_superframe_battery_life_extension(bp.superframe_raw));
        ISO_CHECK(ie_superframe_pan_coordinator(bp.superframe_raw));
        ISO_CHECK(ie_superframe_association_permit(bp.superframe_raw));
        ISO_CHECK_EQ_INT(bp.gts_descriptor_count, 0);
        ISO_CHECK(bp.gts_permit && !bp.gts_has_directions);
        ISO_CHECK_EQ_UINT(bp.short_address_count, 1);
        ISO_CHECK_EQ_UINT(bp.short_addresses[0], 0x1234);
        ISO_CHECK_EQ_UINT(bp.extended_address_count, 1);
        ISO_CHECK(bp.extended_addresses[0] == 0x8877665544332211ull);
        {
            uint8_t want[] = {0xaa, 0xbb};
            ISO_CHECK_MEM_EQ(bp.payload, want, 2);
        }
        ie_beacon_payload_free(&bp);
    }

    /* ── beacon payload with GTS descriptors ──────────────────────────────── */
    {
        uint8_t bytes[] = {0xcf, 0x0f, 0x81, 0x01, 0x67, 0x45, 0x35, 0x00};
        IE_BeaconPayload bp;
        ISO_CHECK_EQ_INT(ie_beacon_payload_parse(bytes, sizeof bytes, &bp),
                         IE_BEACON_OK);
        ISO_CHECK_EQ_INT(ie_superframe_beacon_order(bp.superframe_raw), 15);
        ISO_CHECK_EQ_INT(ie_superframe_order(bp.superframe_raw), 12);
        ISO_CHECK_EQ_INT(bp.gts_descriptor_count, 1);
        ISO_CHECK(bp.gts_has_directions && bp.gts_directions == 0x01);
        ISO_CHECK_EQ_UINT(bp.gts_descriptors[0].short_address, 0x4567);
        ISO_CHECK_EQ_INT(bp.gts_descriptors[0].starting_slot, 5);
        ISO_CHECK_EQ_INT(bp.gts_descriptors[0].length, 3);
        ISO_CHECK_EQ_UINT(bp.short_address_count, 0);
        ISO_CHECK_EQ_UINT(bp.payload_len, 0);
        ie_beacon_payload_free(&bp);
    }

    /* ── truncated beacon payload ─────────────────────────────────────────── */
    {
        uint8_t bytes[] = {0xff, 0xdf, 0x00, 0x10};
        IE_BeaconPayload bp;
        ISO_CHECK_EQ_INT(ie_beacon_payload_parse(bytes, sizeof bytes, &bp),
                         IE_BEACON_ERR_TRUNCATED_FIELD);
    }

    /* ── PAN descriptor from beacon frame ─────────────────────────────────── */
    {
        IE_MacFrame f;
        IE_PanDescriptor pd;
        uint8_t payload[] = {0xff, 0xdf, 0x00, 0x00};
        memset(&f, 0, sizeof f);
        f.frame_control.frame_type = IE_FRAME_BEACON;
        f.frame_control.frame_version = IE_VERSION_2006;
        f.frame_control.destination_address_mode = IE_ADDR_NONE;
        f.frame_control.source_address_mode = IE_ADDR_EXTENDED;
        f.has_sequence_number = 1;
        f.sequence_number = 0x2a;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        f.has_source = 1;
        f.source.mode = IE_ADDR_EXTENDED;
        f.source.extended_addr = 0x8877665544332211ull;
        f.payload = payload;
        f.payload_len = 4;
        ISO_CHECK_EQ_INT(
            ie_pan_descriptor_from_beacon_frame(&f, 15, 0, 244, &pd),
            IE_BEACON_OK);
        ISO_CHECK_EQ_UINT(pd.coordinator_pan_id, 0x1234);
        ISO_CHECK(pd.coordinator_address.mode == IE_ADDR_EXTENDED &&
                  pd.coordinator_address.extended_addr == 0x8877665544332211ull);
        ISO_CHECK_EQ_INT(pd.channel, 15);
        ISO_CHECK_EQ_INT(pd.channel_page, 0);
        ISO_CHECK_EQ_INT(pd.link_quality, 244);
        ISO_CHECK(ie_pan_descriptor_association_permitted(&pd));
        ie_pan_descriptor_free(&pd);
    }

    /* ── PAN descriptor from non-beacon frame is rejected ─────────────────── */
    {
        IE_MacFrame f;
        IE_PanDescriptor pd;
        memset(&f, 0, sizeof f);
        f.frame_control = data_frame_control();
        f.has_source = 1;
        f.source.mode = IE_ADDR_SHORT;
        f.source.short_addr = 0x0001;
        f.has_source_pan_id = 1;
        f.source_pan_id = 0x1234;
        ISO_CHECK_EQ_INT(
            ie_pan_descriptor_from_beacon_frame(&f, 11, 0, 128, &pd),
            IE_BEACON_ERR_EXPECTED_BEACON_FRAME);
    }

    /* ── PAN scan summary filtering and ranking ───────────────────────────── */
    {
        IE_PanDescriptor d[3];
        size_t i;
        memset(d, 0, sizeof d);
        /* closed(0x1001, ch11, lqi180, no assoc), weak(ch12,80,assoc),
         * strong(ch12,220,assoc). superframe 0x4000 | (assoc ? 0x8000 : 0). */
        d[0].channel = 11;
        d[0].link_quality = 180;
        d[0].beacon.superframe_raw = 0x4000;
        d[1].channel = 12;
        d[1].link_quality = 80;
        d[1].beacon.superframe_raw = 0xC000;
        d[2].channel = 12;
        d[2].link_quality = 220;
        d[2].beacon.superframe_raw = 0xC000;
        for (i = 0; i < 3; i++) {
            d[i].coordinator_address.mode = IE_ADDR_EXTENDED;
        }
        ISO_CHECK_EQ_UINT(ie_pan_scan_count_for_channel(d, 3, 12), 2);
        ISO_CHECK_EQ_UINT(ie_pan_scan_association_candidate_count(d, 3), 2);
        ISO_CHECK_EQ_INT(ie_pan_scan_best_candidate_index(d, 3), 2); /* strong */
    }
    {
        IE_PanDescriptor d[1];
        memset(d, 0, sizeof d);
        d[0].channel = 11;
        d[0].link_quality = 240;
        d[0].beacon.superframe_raw = 0x4000; /* not association-permitted */
        ISO_CHECK_EQ_UINT(ie_pan_scan_association_candidate_count(d, 1), 0);
        ISO_CHECK_EQ_INT(ie_pan_scan_best_candidate_index(d, 1), -1);
    }

    /* ── truncated frame ──────────────────────────────────────────────────── */
    {
        uint8_t bytes[] = {0x41};
        IE_MacFrame f;
        ISO_CHECK_EQ_INT(ie_mac_frame_parse(bytes, sizeof bytes, 0, &f),
                         IE_MAC_ERR_TRUNCATED);
    }

    /* ── address mode encoded length ──────────────────────────────────────── */
    ISO_CHECK_EQ_UINT(ie_address_mode_encoded_len(IE_ADDR_NONE), 0);
    ISO_CHECK_EQ_UINT(ie_address_mode_encoded_len(IE_ADDR_SHORT), 2);
    ISO_CHECK_EQ_UINT(ie_address_mode_encoded_len(IE_ADDR_EXTENDED), 8);

    /* ── frame-control encode/parse round-trip ────────────────────────────── */
    {
        IE_FrameControl fc = data_frame_control();
        ISO_CHECK(fc_eq(ie_frame_control_parse(ie_frame_control_encode(fc)),
                        fc));
    }

    return ISO_TEST_RESULT();
}
