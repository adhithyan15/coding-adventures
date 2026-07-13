// Tests for ieee802154-core, mirroring the Rust crate's unit tests, using the
// header-only iso_test.h harness (pure ISO C++17).
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <vector>

#include "ieee802154_core.hpp"

namespace ie = ca::ieee802154_core;
using Bytes = std::vector<std::uint8_t>;

static ie::FrameControl data_frame_control() {
    ie::FrameControl fc;
    fc.frame_type = ie::FrameType::Data;
    fc.pan_id_compression = true;
    fc.destination_address_mode = ie::AddressMode::Short;
    fc.frame_version = ie::FrameVersion::Ieee2006;
    fc.source_address_mode = ie::AddressMode::Short;
    return fc;
}

template <typename F>
static bool throws_mac(F f, ie::MacError want) {
    try {
        f();
        return false;
    } catch (ie::MacError e) {
        return e == want;
    }
}
template <typename F>
static bool throws_beacon(F f, ie::BeaconError want) {
    try {
        f();
        return false;
    } catch (ie::BeaconError e) {
        return e == want;
    }
}

int main() {
    // ── parse short-address data frame without FCS ──────────────────────────
    {
        auto f = ie::MacFrame::parse_without_fcs(
            {0x41, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a, 0x01, 0x02});
        ISO_CHECK(f.frame_control == data_frame_control());
        ISO_CHECK(f.sequence_number == std::optional<std::uint8_t>(7));
        ISO_CHECK(f.destination_pan_id == std::optional<std::uint16_t>(0x1234));
        ISO_CHECK(f.destination == ie::Address::make_short(0x5678));
        ISO_CHECK(f.source_pan_id == std::optional<std::uint16_t>(0x1234));
        ISO_CHECK(f.source == ie::Address::make_short(0x9abc));
        ISO_CHECK(!f.auxiliary_security_header.has_value());
        ISO_CHECK(f.payload == Bytes({0x01, 0x02}));
        ISO_CHECK(!f.fcs.has_value());
    }

    // ── encode short-address data frame without FCS ─────────────────────────
    {
        ie::MacFrame f;
        f.frame_control = data_frame_control();
        f.sequence_number = 7;
        f.destination_pan_id = 0x1234;
        f.destination = ie::Address::make_short(0x5678);
        f.source_pan_id = 0x1234;
        f.source = ie::Address::make_short(0x9abc);
        f.payload = {0x01, 0x02};
        ISO_CHECK(f.encode() ==
                  Bytes({0x41, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a,
                         0x01, 0x02}));
    }

    // ── ack frame ───────────────────────────────────────────────────────────
    {
        auto f = ie::MacFrame::parse_without_fcs({0x02, 0x00, 0x2a});
        ISO_CHECK(f.frame_control.frame_type == ie::FrameType::Acknowledgment);
        ISO_CHECK(f.sequence_number == std::optional<std::uint8_t>(0x2a));
        ISO_CHECK(!f.destination && !f.source);
        ISO_CHECK(f.payload.empty());
    }

    // ── frame with FCS ──────────────────────────────────────────────────────
    {
        auto f = ie::MacFrame::parse_with_fcs({0x02, 0x00, 0x2a, 0xef, 0xbe});
        ISO_CHECK(f.sequence_number == std::optional<std::uint8_t>(0x2a));
        ISO_CHECK(f.fcs == std::optional<std::uint16_t>(0xbeef));
        ISO_CHECK(f.payload.empty());
    }

    // ── summary ─────────────────────────────────────────────────────────────
    {
        ie::MacFrame f;
        f.frame_control = data_frame_control();
        f.frame_control.ack_request = true;
        f.frame_control.frame_pending = true;
        f.sequence_number = 7;
        f.destination_pan_id = 0x1234;
        f.destination = ie::Address::make_short(0x5678);
        f.source_pan_id = 0x1234;
        f.source = ie::Address::make_short(0x9abc);
        f.payload = {0xaa, 0xbb, 0xcc};
        f.fcs = 0xbeef;
        auto s = f.summary();
        ISO_CHECK(s.frame_type == ie::FrameType::Data);
        ISO_CHECK(s.frame_version == ie::FrameVersion::Ieee2006);
        ISO_CHECK(s.ack_request && s.frame_pending && s.pan_id_compression);
        ISO_CHECK(!s.sequence_number_suppressed && s.has_sequence_number);
        ISO_CHECK(s.has_destination && s.has_source && s.has_addressing());
        ISO_CHECK_EQ_UINT(s.payload_len, 3u);
        ISO_CHECK(s.has_payload() && s.has_fcs);
    }
    {
        auto f = ie::MacFrame::parse_without_fcs({0x02, 0x00, 0x2a});
        auto s = f.summary();
        ISO_CHECK(s.frame_type == ie::FrameType::Acknowledgment);
        ISO_CHECK(!s.has_addressing() && !s.has_payload());
        ISO_CHECK(!s.security_enabled && !s.has_auxiliary_security_header);
        ISO_CHECK(s.has_sequence_number && !s.has_fcs);
    }

    // ── sequence number suppression ─────────────────────────────────────────
    {
        auto f = ie::MacFrame::parse_without_fcs(
            {0x41, 0x99, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a});
        ISO_CHECK(f.frame_control.sequence_number_suppression);
        ISO_CHECK(!f.sequence_number.has_value());
        ISO_CHECK(f.payload.empty());
    }

    // ── reserved address mode rejected ──────────────────────────────────────
    ISO_CHECK(throws_mac(
        [] { ie::MacFrame::parse_without_fcs({0x01, 0x04, 0x07}); },
        ie::MacError::ReservedAddressMode));

    // ── aux security header with key index ──────────────────────────────────
    {
        auto f = ie::MacFrame::parse_without_fcs(
            {0x49, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a, 0x0d, 0x44,
             0x33, 0x22, 0x11, 0x02, 0xaa, 0xbb});
        ISO_CHECK(f.auxiliary_security_header.has_value());
        const auto& h = *f.auxiliary_security_header;
        ISO_CHECK(h.security_control.security_level ==
                  ie::SecurityLevel::EncMic32);
        ISO_CHECK(h.security_control.key_identifier_mode ==
                  ie::KeyIdMode::KeyIndex);
        ISO_CHECK(h.frame_counter.has_value() && !h.frame_counter->is_40bit &&
                  h.frame_counter->value == 0x11223344u);
        ISO_CHECK(h.key_identifier.mode == ie::KeyIdMode::KeyIndex &&
                  h.key_identifier.index == 2);
        ISO_CHECK(f.payload == Bytes({0xaa, 0xbb}));
    }

    // ── encode aux security header with key source 8 ────────────────────────
    {
        ie::MacFrame f;
        f.frame_control = data_frame_control();
        f.frame_control.security_enabled = true;
        f.sequence_number = 7;
        f.destination_pan_id = 0x1234;
        f.destination = ie::Address::make_short(0x5678);
        f.source_pan_id = 0x1234;
        f.source = ie::Address::make_short(0x9abc);
        ie::AuxSecurityHeader h;
        h.security_control.security_level = ie::SecurityLevel::EncMic64;
        h.security_control.key_identifier_mode = ie::KeyIdMode::KeySource8;
        h.security_control.frame_counter_size_5 = true;
        h.frame_counter = ie::FrameCounter{true, 0x000102030405ull};
        h.key_identifier.mode = ie::KeyIdMode::KeySource8;
        h.key_identifier.source = {0x10, 0x11, 0x12, 0x13,
                                   0x14, 0x15, 0x16, 0x17};
        h.key_identifier.index = 0x22;
        f.auxiliary_security_header = h;
        f.payload = {0xaa};
        ISO_CHECK(f.encode() ==
                  Bytes({0x49, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a,
                         0x5e, 0x05, 0x04, 0x03, 0x02, 0x01, 0x10, 0x11, 0x12,
                         0x13, 0x14, 0x15, 0x16, 0x17, 0x22, 0xaa}));
    }

    // ── encode security enabled without aux header ──────────────────────────
    {
        ie::MacFrame f;
        f.frame_control = data_frame_control();
        f.frame_control.security_enabled = true;
        f.sequence_number = 7;
        f.destination_pan_id = 0x1234;
        f.destination = ie::Address::make_short(0x5678);
        f.source_pan_id = 0x1234;
        f.source = ie::Address::make_short(0x9abc);
        ISO_CHECK(throws_mac([&] { f.encode(); },
                             ie::MacError::MissingAuxiliarySecurityHeader));
    }

    // ── security level helpers ──────────────────────────────────────────────
    ISO_CHECK(!ie::encrypts(ie::SecurityLevel::Mic64));
    ISO_CHECK_EQ_UINT(ie::mic_len(ie::SecurityLevel::Mic64), 8u);
    ISO_CHECK(ie::encrypts(ie::SecurityLevel::EncMic128));
    ISO_CHECK_EQ_UINT(ie::mic_len(ie::SecurityLevel::EncMic128), 16u);

    // ── beacon payload with pending addresses ───────────────────────────────
    {
        auto bp = ie::BeaconPayload::parse(
            Bytes({0xff, 0xdf, 0x80, 0x11, 0x34, 0x12, 0x11, 0x22, 0x33, 0x44,
                   0x55, 0x66, 0x77, 0x88, 0xaa, 0xbb}));
        ISO_CHECK_EQ_UINT(bp.superframe.raw, 0xdfffu);
        ISO_CHECK_EQ_INT(bp.superframe.beacon_order(), 15);
        ISO_CHECK_EQ_INT(bp.superframe.superframe_order(), 15);
        ISO_CHECK(bp.superframe.battery_life_extension());
        ISO_CHECK(bp.superframe.pan_coordinator());
        ISO_CHECK(bp.superframe.association_permit());
        ISO_CHECK_EQ_INT(bp.gts.descriptor_count, 0);
        ISO_CHECK(bp.gts.permit && !bp.gts.directions.has_value());
        ISO_CHECK(bp.short_addresses == std::vector<std::uint16_t>({0x1234}));
        ISO_CHECK(bp.extended_addresses ==
                  std::vector<std::uint64_t>({0x8877665544332211ull}));
        ISO_CHECK(bp.payload == Bytes({0xaa, 0xbb}));
    }

    // ── beacon payload with GTS descriptors ─────────────────────────────────
    {
        auto bp = ie::BeaconPayload::parse(
            Bytes({0xcf, 0x0f, 0x81, 0x01, 0x67, 0x45, 0x35, 0x00}));
        ISO_CHECK_EQ_INT(bp.superframe.beacon_order(), 15);
        ISO_CHECK_EQ_INT(bp.superframe.superframe_order(), 12);
        ISO_CHECK_EQ_INT(bp.gts.descriptor_count, 1);
        ISO_CHECK(bp.gts.directions == std::optional<std::uint8_t>(0x01));
        ISO_CHECK_EQ_UINT(bp.gts.descriptors.size(), 1u);
        ISO_CHECK(bp.gts.descriptors[0].short_address == 0x4567 &&
                  bp.gts.descriptors[0].starting_slot == 5 &&
                  bp.gts.descriptors[0].length == 3);
        ISO_CHECK(bp.short_addresses.empty() && bp.payload.empty());
    }

    // ── truncated beacon payload ────────────────────────────────────────────
    ISO_CHECK(throws_beacon(
        [] { ie::BeaconPayload::parse(Bytes({0xff, 0xdf, 0x00, 0x10})); },
        ie::BeaconError::TruncatedField));

    // ── PAN descriptor from beacon frame ────────────────────────────────────
    {
        ie::MacFrame f;
        f.frame_control.frame_type = ie::FrameType::Beacon;
        f.frame_control.frame_version = ie::FrameVersion::Ieee2006;
        f.frame_control.destination_address_mode = ie::AddressMode::None;
        f.frame_control.source_address_mode = ie::AddressMode::Extended;
        f.sequence_number = 0x2a;
        f.source_pan_id = 0x1234;
        f.source = ie::Address::make_extended(0x8877665544332211ull);
        f.payload = {0xff, 0xdf, 0x00, 0x00};
        auto pd = ie::PanDescriptor::from_beacon_frame(f, 15, 0, 244);
        ISO_CHECK_EQ_UINT(pd.coordinator_pan_id, 0x1234u);
        ISO_CHECK(pd.coordinator_address ==
                  ie::Address::make_extended(0x8877665544332211ull));
        ISO_CHECK_EQ_INT(pd.channel, 15);
        ISO_CHECK_EQ_INT(pd.link_quality, 244);
        ISO_CHECK(pd.association_permitted());
    }

    // ── PAN descriptor from non-beacon frame rejected ───────────────────────
    {
        ie::MacFrame f;
        f.frame_control = data_frame_control();
        f.source = ie::Address::make_short(0x0001);
        f.source_pan_id = 0x1234;
        f.destination = ie::Address::make_short(0xffff);
        f.destination_pan_id = 0x1234;
        ISO_CHECK(throws_beacon(
            [&] { ie::PanDescriptor::from_beacon_frame(f, 11, 0, 128); },
            ie::BeaconError::ExpectedBeaconFrame));
    }

    // ── PAN scan summary filtering and ranking ──────────────────────────────
    {
        auto mk = [](std::uint16_t pan, std::uint8_t ch, std::uint8_t lqi,
                     bool assoc) {
            ie::PanDescriptor pd;
            pd.coordinator_pan_id = pan;
            pd.coordinator_address = ie::Address::make_extended(
                0x8877665544332211ull + pan);
            pd.channel = ch;
            pd.channel_page = 0;
            pd.link_quality = lqi;
            pd.beacon.superframe.raw =
                static_cast<std::uint16_t>(0x4000 | (assoc ? 0x8000 : 0));
            return pd;
        };
        ie::PanScanSummary sum;
        sum.scanned_at_ms = 5000;
        sum.descriptors = {mk(0x1001, 11, 180, false), mk(0x1002, 12, 80, true),
                           mk(0x1003, 12, 220, true)};
        ISO_CHECK_EQ_UINT(sum.scanned_at_ms, 5000u);
        ISO_CHECK_EQ_UINT(sum.len(), 3u);
        ISO_CHECK(!sum.is_empty());
        ISO_CHECK_EQ_UINT(sum.descriptors_for_channel(12).size(), 2u);
        ISO_CHECK_EQ_UINT(sum.association_candidates().size(), 2u);
        const auto* best = sum.best_association_candidate();
        ISO_CHECK(best != nullptr && best->coordinator_pan_id == 0x1003);
    }
    {
        auto mk = [](std::uint8_t lqi) {
            ie::PanDescriptor pd;
            pd.channel = 11;
            pd.link_quality = lqi;
            pd.beacon.superframe.raw = 0x4000;  // not association-permitted
            return pd;
        };
        ie::PanScanSummary sum;
        sum.descriptors = {mk(240)};
        ISO_CHECK_EQ_UINT(sum.association_candidates().size(), 0u);
        ISO_CHECK(sum.best_association_candidate() == nullptr);
    }

    // ── truncated frame ─────────────────────────────────────────────────────
    ISO_CHECK(throws_mac([] { ie::MacFrame::parse_without_fcs({0x41}); },
                         ie::MacError::Truncated));

    // ── address mode encoded length ─────────────────────────────────────────
    ISO_CHECK_EQ_UINT(ie::encoded_len(ie::AddressMode::None), 0u);
    ISO_CHECK_EQ_UINT(ie::encoded_len(ie::AddressMode::Short), 2u);
    ISO_CHECK_EQ_UINT(ie::encoded_len(ie::AddressMode::Extended), 8u);

    // ── frame-control round-trip ────────────────────────────────────────────
    {
        auto fc = data_frame_control();
        ISO_CHECK(ie::FrameControl::parse(fc.encode()) == fc);
    }

    return ISO_TEST_RESULT();
}
