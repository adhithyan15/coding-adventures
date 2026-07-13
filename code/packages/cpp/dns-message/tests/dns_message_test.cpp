// Tests for dns-message, using the header-only iso_test.h harness (pure ISO).
// Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "dns_message.hpp"

namespace dm = ca::dns_message;

// The canonical `info.cern.ch A IN` query.
static std::vector<std::uint8_t> info_cern_query() {
    return {0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x04, 'i',  'n',  'f',  'o',  0x04, 'c',  'e',
            'r',  'n',  0x02, 'c',  'h',  0x00, 0x00, 0x01, 0x00, 0x01};
}

// Did parsing throw an Error of the given kind?
static bool parse_throws(const std::vector<std::uint8_t> &bytes,
                         dm::ErrorKind kind) {
    try {
        dm::parse_message(bytes);
    } catch (const dm::Error &e) {
        return e.kind() == kind;
    }
    return false;
}

int main() {
    using dm::Class;
    using dm::DnsName;
    using dm::Message;
    using dm::RecordData;
    using dm::RecordType;
    using dm::ResourceRecord;

    // ── build + serialize matches the spec bytes ─────────────────────────────
    {
        Message q = dm::build_query(0x1234, DnsName::from_ascii("info.cern.ch"),
                                    RecordType{RecordType::A, 0});
        ISO_CHECK(dm::serialize_message(q) == info_cern_query());
    }

    // ── parse round-trip ─────────────────────────────────────────────────────
    {
        Message p = dm::parse_message(info_cern_query());
        ISO_CHECK(p.header.id == 0x1234);
        ISO_CHECK(!p.header.flags.is_response);
        ISO_CHECK(p.header.flags.recursion_desired);
        ISO_CHECK(p.questions.size() == 1);
        ISO_CHECK(p.questions[0].name.to_string() == "info.cern.ch");
        ISO_CHECK(p.questions[0].qtype.kind == RecordType::A);
        ISO_CHECK(p.questions[0].qclass.kind == Class::IN);
    }

    // ── root name + trailing dot ─────────────────────────────────────────────
    {
        DnsName root = DnsName::from_ascii(".");
        DnsName trailing = DnsName::from_ascii("example.com.");
        ISO_CHECK(root.is_root());
        ISO_CHECK(root.to_string() == ".");
        ISO_CHECK(!trailing.is_root());
        ISO_CHECK(trailing.to_string() == "example.com");
    }

    // ── name construction errors (throw Error of the right kind) ─────────────
    {
        bool threw = false;
        try {
            DnsName::from_ascii("bad..example");
        } catch (const dm::Error &e) {
            threw = e.kind() == dm::ErrorKind::Unsupported;
        }
        ISO_CHECK(threw);

        std::string label63(63, 'a');
        std::string too_long =
            label63 + "." + label63 + "." + label63 + "." + label63;
        threw = false;
        try {
            DnsName::from_ascii(too_long);
        } catch (const dm::Error &e) {
            threw = e.kind() == dm::ErrorKind::NameTooLong;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            DnsName::from_ascii(std::string(64, 'a') + ".example");
        } catch (const dm::Error &e) {
            threw = e.kind() == dm::ErrorKind::LabelTooLong && e.detail() == 64;
        }
        ISO_CHECK(threw);

        ISO_CHECK(!parse_throws(info_cern_query(), dm::ErrorKind::NonAsciiLabel));
        threw = false;
        try {
            DnsName::from_ascii("caf\xc3\xa9.example");
        } catch (const dm::Error &e) {
            threw = e.kind() == dm::ErrorKind::NonAsciiLabel;
        }
        ISO_CHECK(threw);
    }

    // ── compressed A response ────────────────────────────────────────────────
    {
        std::vector<std::uint8_t> bytes = info_cern_query();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        std::vector<std::uint8_t> answer = {0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01,
                                            0x00, 0x00, 0x01, 0x2c, 0x00, 0x04,
                                            188,  184,  21,   108};
        bytes.insert(bytes.end(), answer.begin(), answer.end());

        Message p = dm::parse_message(bytes);
        ISO_CHECK(p.is_success());
        ISO_CHECK(p.answers.size() == 1);
        ISO_CHECK(p.answers[0].name.to_string() == "info.cern.ch");
        ISO_CHECK(p.answers[0].ttl == 300);
        auto v4 = p.ipv4_answers();
        ISO_CHECK(v4.size() == 1);
        ISO_CHECK((v4[0] == std::array<std::uint8_t, 4>{188, 184, 21, 108}));
    }

    // ── compressed CNAME target ──────────────────────────────────────────────
    {
        std::vector<std::uint8_t> bytes = info_cern_query();
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        std::vector<std::uint8_t> answer = {
            0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x3c,
            0x00, 0x08, 0x05, 'a',  'l',  'i',  'a',  's',  0xc0, 0x11};
        bytes.insert(bytes.end(), answer.begin(), answer.end());

        Message p = dm::parse_message(bytes);
        ISO_CHECK(p.answers[0].rrtype.kind == RecordType::CNAME);
        ISO_CHECK(p.answers[0].data.kind == RecordData::CNAME);
        ISO_CHECK(p.answers[0].data.name.to_string() == "alias.cern.ch");
    }

    // ── AAAA round-trip ──────────────────────────────────────────────────────
    {
        Message m;
        m.header.flags = dm::Flags::query();
        m.header.flags.is_response = true;
        m.header.flags.recursion_available = true;
        ResourceRecord r;
        r.name = DnsName::from_ascii("example.com");
        r.rrtype = RecordType{RecordType::AAAA, 0};
        r.rclass = Class{Class::IN, 0};
        r.ttl = 10;
        r.data.kind = RecordData::AAAA;
        r.data.aaaa = {0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1};
        m.answers.push_back(r);

        Message p = dm::parse_message(dm::serialize_message(m));
        ISO_CHECK(p.ipv6_answers().size() == 1);
        ISO_CHECK(p.answers[0] == r);
        ISO_CHECK(p.answers[0].ttl == 10);
    }

    // ── unknown record data preserved ────────────────────────────────────────
    {
        Message m;
        m.header.flags = dm::Flags::query();
        ResourceRecord r;
        r.name = DnsName::from_ascii("example.com");
        r.rrtype = RecordType{RecordType::Unknown, 65};
        r.rclass = Class{Class::IN, 0};
        r.ttl = 1;
        r.data.kind = RecordData::RAW;
        r.data.raw = {1, 2, 3};
        m.answers.push_back(r);

        Message p = dm::parse_message(dm::serialize_message(m));
        ISO_CHECK(p.answers[0] == r);
    }

    // ── PTR + SRV (DNS-SD) round-trip ────────────────────────────────────────
    {
        Message m;
        m.header.flags = dm::Flags::query();
        m.header.flags.is_response = true;
        m.header.flags.authoritative_answer = true;
        m.header.flags.recursion_desired = false;

        ResourceRecord ptr;
        ptr.name = DnsName::from_ascii("_hue._tcp.local");
        ptr.rrtype = RecordType{RecordType::PTR, 0};
        ptr.rclass = Class{Class::IN, 0};
        ptr.ttl = 120;
        ptr.data.kind = RecordData::PTR;
        ptr.data.name = DnsName::from_ascii("bridge-1._hue._tcp.local");
        m.answers.push_back(ptr);

        ResourceRecord srv;
        srv.name = DnsName::from_ascii("bridge-1._hue._tcp.local");
        srv.rrtype = RecordType{RecordType::SRV, 0};
        srv.rclass = Class{Class::IN, 0};
        srv.ttl = 120;
        srv.data.kind = RecordData::SRV;
        srv.data.srv.priority = 0;
        srv.data.srv.weight = 0;
        srv.data.srv.port = 443;
        srv.data.srv.target = DnsName::from_ascii("bridge-1.local");
        m.additionals.push_back(srv);

        Message p = dm::parse_message(dm::serialize_message(m));
        ISO_CHECK(p.answers[0].data.name.to_string() ==
                  "bridge-1._hue._tcp.local");
        ISO_CHECK(p.additionals[0].data.kind == RecordData::SRV);
        ISO_CHECK(p.additionals[0].data.srv.port == 443);
        ISO_CHECK(p.additionals[0].data.srv.target.to_string() ==
                  "bridge-1.local");
    }

    // ── first_answer_of_type ─────────────────────────────────────────────────
    {
        Message m;
        m.header.flags = dm::Flags::query();
        ResourceRecord r;
        r.name = DnsName::from_ascii("example.com");
        r.rrtype = RecordType{RecordType::A, 0};
        r.data.kind = RecordData::A;
        m.answers.push_back(r);
        ISO_CHECK(m.first_answer_of_type(RecordType{RecordType::A, 0}) !=
                  nullptr);
        ISO_CHECK(m.first_answer_of_type(RecordType{RecordType::AAAA, 0}) ==
                  nullptr);
    }

    // ── unknown opcode + response code ───────────────────────────────────────
    {
        std::vector<std::uint8_t> bytes = {0x00, 0x01, 0xf0, 0x0f, 0x00, 0x00,
                                           0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        Message p = dm::parse_message(bytes);
        ISO_CHECK(p.header.flags.opcode.kind == dm::Opcode::Unknown &&
                  p.header.flags.opcode.value == 14);
        ISO_CHECK(p.header.flags.response_code.kind ==
                      dm::ResponseCode::Unknown &&
                  p.header.flags.response_code.value == 15);
    }

    // ── error cases ──────────────────────────────────────────────────────────
    ISO_CHECK(parse_throws(std::vector<std::uint8_t>(11, 0),
                           dm::ErrorKind::TruncatedHeader));
    {
        std::vector<std::uint8_t> t = info_cern_query();
        t.pop_back();
        ISO_CHECK(parse_throws(t, dm::ErrorKind::UnexpectedEof));
    }
    ISO_CHECK(parse_throws(
        {0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01},
        dm::ErrorKind::PointerLoop));
    {
        std::vector<std::uint8_t> oob = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                         0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                         0xc0, 0xff, 0x00, 0x01, 0x00, 0x01};
        bool ok = false;
        try {
            dm::parse_message(oob);
        } catch (const dm::Error &e) {
            ok = e.kind() == dm::ErrorKind::PointerOutOfBounds &&
                 e.detail() == 255;
        }
        ISO_CHECK(ok);
    }
    ISO_CHECK(parse_throws({0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                            0x00, 0x00, 0x00, 0xc0},
                           dm::ErrorKind::UnexpectedEof));
    ISO_CHECK(parse_throws(
        {0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x01, 0xff, 0x00, 0x00, 0x01, 0x00, 0x01},
        dm::ErrorKind::NonAsciiLabel));
    ISO_CHECK(parse_throws(
        {0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x40, 0x00, 0x01, 0x00, 0x01},
        dm::ErrorKind::Unsupported));

    // wire name longer than 255 (4 x 63-byte labels)
    {
        std::vector<std::uint8_t> big = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                         0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        for (int r = 0; r < 4; ++r) {
            big.push_back(63);
            for (int k = 0; k < 63; ++k) big.push_back('a');
        }
        big.insert(big.end(), {0x00, 0x00, 0x01, 0x00, 0x01});
        ISO_CHECK(parse_throws(big, dm::ErrorKind::NameTooLong));
    }

    // excessive pointer chain trips the hop cap
    {
        std::vector<std::uint8_t> chain = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                           0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        for (std::size_t hop = 0; hop <= dm::limits::kMaxNamePointerHops;
             ++hop) {
            std::size_t target = 12 + ((hop + 1) * 2);
            chain.push_back(
                static_cast<std::uint8_t>(0xc0 | ((target >> 8) & 0x3f)));
            chain.push_back(static_cast<std::uint8_t>(target & 0xff));
        }
        ISO_CHECK(parse_throws(chain, dm::ErrorKind::PointerLoop));
    }

    // ── serialize-side guards ────────────────────────────────────────────────
    {
        Message m;
        m.header.flags = dm::Flags::query();
        m.questions.resize(0x10000);  // > u16::MAX
        bool ok = false;
        try {
            dm::serialize_message(m);
        } catch (const dm::Error &e) {
            ok = e.kind() == dm::ErrorKind::InvalidSectionCount;
        }
        ISO_CHECK(ok);
    }
    {
        Message m;
        m.header.flags = dm::Flags::query();
        ResourceRecord r;
        r.name = DnsName::from_ascii(".");
        r.rrtype = RecordType{RecordType::Unknown, 65000};
        r.rclass = Class{Class::IN, 0};
        r.data.kind = RecordData::RAW;
        r.data.raw.assign(0x10000, 0);  // 65536 > 65535
        m.answers.push_back(r);
        bool ok = false;
        try {
            dm::serialize_message(m);
        } catch (const dm::Error &e) {
            ok = e.kind() == dm::ErrorKind::Unsupported;
        }
        ISO_CHECK(ok);
    }

    // ── question record types round-trip ─────────────────────────────────────
    {
        RecordType::Kind kinds[] = {RecordType::NS,   RecordType::CNAME,
                                    RecordType::SOA,  RecordType::PTR,
                                    RecordType::MX,   RecordType::TXT,
                                    RecordType::AAAA, RecordType::SRV};
        for (RecordType::Kind k : kinds) {
            Message q = dm::build_query(5, DnsName::from_ascii("example.com"),
                                        RecordType{k, 0});
            Message p = dm::parse_message(dm::serialize_message(q));
            ISO_CHECK(p.questions[0].qtype.kind == k);
        }
    }

    return ISO_TEST_RESULT();
}
