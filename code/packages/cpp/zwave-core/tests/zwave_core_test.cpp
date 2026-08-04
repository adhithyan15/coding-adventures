// Tests for zwave-core, using the header-only iso_test.h harness (pure ISO).
// Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "zwave_core.hpp"

namespace zw = ca::zwave_core;

// Did `fn` throw a zw::Error of the given kind?
template <class Fn>
static bool throws_kind(Fn fn, zw::ErrorKind kind) {
    try {
        fn();
    } catch (const zw::Error &e) {
        return e.kind() == kind;
    }
    return false;
}

int main() {
    using Bytes = std::vector<std::uint8_t>;

    // ── node id ranges ───────────────────────────────────────────────────────
    {
        ISO_CHECK(zw::NodeId::classic(1) == (zw::NodeId{zw::NodeId::Classic, 1}));
        ISO_CHECK(throws_kind([] { zw::NodeId::classic(0); },
                              zw::ErrorKind::InvalidClassicNodeId));
        ISO_CHECK(throws_kind([] { zw::NodeId::classic(233); },
                              zw::ErrorKind::InvalidClassicNodeId));
        ISO_CHECK(zw::NodeId::long_range(4000).value == 4000);
        ISO_CHECK(throws_kind([] { zw::NodeId::long_range(4001); },
                              zw::ErrorKind::InvalidLongRangeNodeId));
        ISO_CHECK(throws_kind([] { zw::NodeId::long_range(0); },
                              zw::ErrorKind::InvalidLongRangeNodeId));
    }

    // ── home id big-endian ───────────────────────────────────────────────────
    {
        auto b = zw::home_id_to_be_bytes(0xDEADBEEFu);
        ISO_CHECK((b == std::array<std::uint8_t, 4>{0xDE, 0xAD, 0xBE, 0xEF}));
    }

    // ── region profiles ──────────────────────────────────────────────────────
    {
        ISO_CHECK(!zw::supports_long_range(zw::RegionProfile::UnitedStates));
        ISO_CHECK(
            zw::supports_long_range(zw::RegionProfile::UnitedStatesLongRange));
        ISO_CHECK_STR_EQ(
            zw::band_description(zw::RegionProfile::EuropeLongRange),
            "EU Z-Wave Long Range");
    }

    // ── command-class ids ────────────────────────────────────────────────────
    {
        ISO_CHECK(zw::kSwitchBinary.value == 0x25);
        ISO_CHECK(zw::kBattery.value == 0x80);
        ISO_CHECK(zw::kSecurity2.value == 0x9f);
        ISO_CHECK(zw::CommandClassId(0x25).encoded_len() == 1);
        ISO_CHECK(zw::CommandClassId(0xf100).encoded_len() == 2);
        ISO_CHECK(zw::kSwitchBinary.is_actuator());
        ISO_CHECK(!zw::kSensorBinary.is_actuator());
        ISO_CHECK(zw::kSensorMultilevel.is_sensor());
        ISO_CHECK(zw::kSecurity2.is_security());
    }

    // ── network summary ──────────────────────────────────────────────────────
    {
        auto s = zw::NetworkSummary::from_parts(
            zw::RegionProfile::UnitedStatesLongRange,
            {zw::NodeId::classic(2), zw::NodeId::long_range(2001)},
            {zw::kSwitchBinary, zw::kSensorMultilevel, zw::kSecurity2});
        zw::NetworkSummary expect;
        expect.region = zw::RegionProfile::UnitedStatesLongRange;
        expect.supports_long_range = true;
        expect.classic_nodes = 1;
        expect.long_range_nodes = 1;
        expect.command_class_entries = 3;
        expect.actuator_command_classes = 1;
        expect.sensor_command_classes = 1;
        expect.security_command_classes = 1;
        ISO_CHECK(s == expect);
        ISO_CHECK(s.has_nodes() && s.has_long_range_nodes() &&
                  s.has_security());

        auto empty =
            zw::NetworkSummary::from_parts(zw::RegionProfile::Europe, {}, {});
        ISO_CHECK(!empty.supports_long_range && !empty.has_nodes() &&
                  !empty.has_security());
    }

    // ── command-class frame round-trips ──────────────────────────────────────
    {
        zw::CommandClassFrame f{zw::kSwitchBinary, 0x01, {0xff}};
        Bytes enc = f.encode();
        ISO_CHECK(enc == (Bytes{0x25, 0x01, 0xff}));
        ISO_CHECK(zw::CommandClassFrame::parse(enc) == f);
    }
    {
        zw::CommandClassFrame f{zw::CommandClassId(0xf100), 0x02, {0x01, 0x02}};
        ISO_CHECK(zw::CommandClassId(0xf100).encoded_len() == 2);
        Bytes enc = f.encode();
        ISO_CHECK(enc == (Bytes{0xf1, 0x00, 0x02, 0x01, 0x02}));
        ISO_CHECK(zw::CommandClassFrame::parse(enc) == f);
    }
    {
        ISO_CHECK(throws_kind([] { zw::CommandClassFrame::parse(Bytes{0xf1}); },
                              zw::ErrorKind::Truncated));
        bool ok = false;
        try {
            zw::CommandClassFrame::parse(Bytes{0x25});
        } catch (const zw::Error &e) {
            ok = e.kind() == zw::ErrorKind::Truncated && e.a() == 2 &&
                 e.b() == 1;
        }
        ISO_CHECK(ok);
    }

    // ── command-class frame summary ──────────────────────────────────────────
    {
        std::vector<zw::CommandClassFrame> frames = {
            {zw::kBasic, 0x01, {0xff}},
            {zw::kSecurity2, 0x02, {0x01, 0x02}},
            {zw::CommandClassId(0xf100), 0x03, {0x55}}};
        auto s = zw::CommandClassFrameSummary::from_frames(frames);
        zw::CommandClassFrameSummary expect;
        expect.frame_count = 3;
        expect.short_command_class_frames = 2;
        expect.extended_command_class_frames = 1;
        expect.security_2_frames = 1;
        expect.total_payload_bytes = 4;
        expect.max_payload_bytes = 2;
        ISO_CHECK(s == expect);
        ISO_CHECK(s.has_extended_command_classes() &&
                  s.has_security_2_frames() && !s.is_empty());
    }

    // ── serial frame round-trip + checksum ───────────────────────────────────
    {
        zw::SerialFrame f{zw::SerialFrameType::Request, 0x13, {0x02, 0x25, 0x01}};
        Bytes enc = f.encode();
        ISO_CHECK(enc[0] == zw::kSof);
        ISO_CHECK(zw::SerialFrame::parse(enc) == f);
        auto sum = zw::SerialFrameSummary::from_frame(f);
        ISO_CHECK((sum == zw::SerialFrameSummary{zw::SerialFrameType::Request,
                                                 0x13, 3}));
        ISO_CHECK(sum.is_request() && !sum.is_response() && sum.has_payload() &&
                  sum.is_function(0x13) && !sum.is_function(0x02));
    }
    {
        zw::SerialFrame f{zw::SerialFrameType::Response, 0x02, {0x01, 0x02}};
        Bytes enc = f.encode();
        enc.back() ^= 0x01;
        ISO_CHECK(throws_kind([&] { zw::SerialFrame::parse(enc); },
                              zw::ErrorKind::ChecksumMismatch));
    }
    {
        ISO_CHECK(throws_kind(
            [] {
                zw::SerialFrame::parse(Bytes{0x02, 0x03, 0x00, 0x13, 0x00});
            },
            zw::ErrorKind::MissingStartOfFrame));
        ISO_CHECK(throws_kind(
            [] { zw::SerialFrame::parse(Bytes{0x01, 0x03, 0x00}); },
            zw::ErrorKind::Truncated));
    }

    // ── serial frame batch summary ───────────────────────────────────────────
    {
        std::vector<zw::SerialFrame> frames = {
            {zw::SerialFrameType::Request, 0x13, {0x02, 0x25, 0x01}},
            {zw::SerialFrameType::Response, 0x02, {0x01}}};
        auto s = zw::SerialFrameBatchSummary::from_frames(frames);
        ISO_CHECK((s == zw::SerialFrameBatchSummary{2, 1, 1, 4, 3}));
        ISO_CHECK(s.has_requests() && s.has_responses() && !s.is_empty());
    }

    // ── controller readiness: ready ──────────────────────────────────────────
    {
        auto net = zw::NetworkSummary::from_parts(
            zw::RegionProfile::UnitedStatesLongRange,
            {zw::NodeId::classic(2), zw::NodeId::long_range(2001)},
            {zw::kSwitchBinary});
        std::vector<zw::CommandClassFrame> cframes = {
            {zw::kSecurity2, 0x02, {0x01}}};
        auto cfs = zw::CommandClassFrameSummary::from_frames(cframes);
        std::vector<zw::SerialFrame> sframes = {
            {zw::SerialFrameType::Request, 0x13, {0x02, 0x25, 0x01}},
            {zw::SerialFrameType::Response, 0x02, {0x01}}};
        auto sfs = zw::SerialFrameBatchSummary::from_frames(sframes);
        auto r = zw::ControllerReadinessSummary::from_summaries(net, cfs, sfs);
        ISO_CHECK(r.is_ready());
        ISO_CHECK(!r.needs_node_discovery() &&
                  !r.needs_command_class_interview() &&
                  !r.needs_serial_probe() && !r.waiting_for_serial_response() &&
                  !r.needs_region_review());
    }
    // ── controller readiness: blocked ────────────────────────────────────────
    {
        auto net = zw::NetworkSummary::from_parts(
            zw::RegionProfile::UnitedStates, {zw::NodeId::long_range(2001)}, {});
        zw::CommandClassFrameSummary cfs;  // default (empty)
        std::vector<zw::SerialFrame> sframes = {
            {zw::SerialFrameType::Request, 0x13, {0x02, 0x25, 0x01}}};
        auto sfs = zw::SerialFrameBatchSummary::from_frames(sframes);
        auto r = zw::ControllerReadinessSummary::from_summaries(net, cfs, sfs);
        ISO_CHECK(!r.is_ready());
        ISO_CHECK(r.needs_command_class_interview() &&
                  r.waiting_for_serial_response() && r.needs_region_review() &&
                  !r.has_security_coverage);
    }

    // ── checksum helper ──────────────────────────────────────────────────────
    {
        Bytes data = {0x03, 0x00, 0x13};
        ISO_CHECK(zw::serial_checksum(data.data(), data.size()) ==
                  static_cast<std::uint8_t>(0xff ^ 0x03 ^ 0x00 ^ 0x13));
    }

    return ISO_TEST_RESULT();
}
