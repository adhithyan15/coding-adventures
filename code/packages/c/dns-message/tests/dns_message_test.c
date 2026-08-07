/* Tests for dns-message, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests, including the canonical
 * info.cern.ch query and compressed-response fixtures. */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "dns_message.h"

/* The canonical `info.cern.ch A IN` query, id 0x1234, RD set. */
static const uint8_t INFO_CERN_QUERY[] = {
    0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x04, 'i',  'n',  'f',  'o',  0x04, 'c',  'e',
    'r',  'n',  0x02, 'c',  'h',  0x00, 0x00, 0x01, 0x00, 0x01};

/* Parse `bytes` and assert the decoder returns `kind`. */
static void expect_parse_err(const uint8_t *bytes, size_t len,
                             DnsErrorKind kind) {
    DnsMessage m;
    DnsError e = dns_parse_message(bytes, len, &m);
    ISO_CHECK_MSG(e.kind == kind, "parse error kind mismatch");
    if (e.kind == DNS_OK) dns_message_free(&m);
}

/* True if `name` renders to `expect` (frees the rendered string). */
static int name_is(const DnsName *name, const char *expect) {
    char *s = dns_name_to_string(name);
    int ok;
    if (s == NULL) return 0;
    ok = strcmp(s, expect) == 0;
    free(s);
    return ok;
}

int main(void) {
    /* ── build + serialize a query matches the spec bytes ──────────────────*/
    {
        DnsName name;
        DnsMessage q;
        uint8_t *bytes;
        size_t n;
        ISO_CHECK(dns_name_from_ascii("info.cern.ch", &name).kind == DNS_OK);
        ISO_CHECK(dns_build_query(0x1234, name, dns_record_type_known(DNS_TYPE_A),
                                  &q)
                      .kind == DNS_OK);
        ISO_CHECK(dns_serialize_message(&q, &bytes, &n).kind == DNS_OK);
        ISO_CHECK(n == sizeof INFO_CERN_QUERY);
        ISO_CHECK_MEM_EQ(bytes, INFO_CERN_QUERY, sizeof INFO_CERN_QUERY);
        free(bytes);
        dns_message_free(&q);
    }

    /* ── parse round-trip ──────────────────────────────────────────────────*/
    {
        DnsMessage p;
        ISO_CHECK(dns_parse_message(INFO_CERN_QUERY, sizeof INFO_CERN_QUERY, &p)
                      .kind == DNS_OK);
        ISO_CHECK(p.header.id == 0x1234);
        ISO_CHECK(!p.header.flags.is_response);
        ISO_CHECK(p.header.flags.recursion_desired);
        ISO_CHECK(p.n_questions == 1);
        ISO_CHECK(name_is(&p.questions[0].name, "info.cern.ch"));
        ISO_CHECK(p.questions[0].qtype.kind == DNS_TYPE_A);
        ISO_CHECK(p.questions[0].qclass.kind == DNS_CLASS_IN);
        dns_message_free(&p);
    }

    /* ── root names and trailing dot ───────────────────────────────────────*/
    {
        DnsName root, trailing;
        ISO_CHECK(dns_name_from_ascii(".", &root).kind == DNS_OK);
        ISO_CHECK(dns_name_from_ascii("example.com.", &trailing).kind == DNS_OK);
        ISO_CHECK(dns_name_is_root(&root));
        ISO_CHECK(name_is(&root, "."));
        ISO_CHECK(!dns_name_is_root(&trailing));
        ISO_CHECK(name_is(&trailing, "example.com"));
        dns_name_free(&root);
        dns_name_free(&trailing);
    }

    /* ── name construction errors ──────────────────────────────────────────*/
    {
        DnsName n;
        ISO_CHECK(dns_name_from_ascii("bad..example", &n).kind ==
                  DNS_ERR_UNSUPPORTED);
        /* 4 * 63-char labels -> encoded length exceeds 255 */
        {
            char big[300];
            size_t i;
            for (i = 0; i < 63; i++) big[i] = 'a';
            big[63] = '.';
            memcpy(big + 64, big, 63);
            big[127] = '.';
            memcpy(big + 128, big, 63);
            big[191] = '.';
            memcpy(big + 192, big, 63);
            big[255] = '\0';
            ISO_CHECK(dns_name_from_ascii(big, &n).kind == DNS_ERR_NAME_TOO_LONG);
        }
        /* a 64-char label */
        {
            char lbl[80];
            size_t i;
            for (i = 0; i < 64; i++) lbl[i] = 'a';
            memcpy(lbl + 64, ".example", 9); /* includes NUL */
            {
                DnsError e = dns_name_from_ascii(lbl, &n);
                ISO_CHECK(e.kind == DNS_ERR_LABEL_TOO_LONG && e.detail == 64);
            }
        }
        /* non-ASCII */
        ISO_CHECK(dns_name_from_ascii("caf\xc3\xa9.example", &n).kind ==
                  DNS_ERR_NON_ASCII_LABEL);
    }

    /* ── compressed A response (pointer to the question name) ──────────────*/
    {
        uint8_t bytes[sizeof INFO_CERN_QUERY + 16];
        DnsMessage p;
        uint8_t v4[4][4];
        size_t got;
        memcpy(bytes, INFO_CERN_QUERY, sizeof INFO_CERN_QUERY);
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01; /* one answer */
        {
            static const uint8_t answer[] = {0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01,
                                             0x00, 0x00, 0x01, 0x2c, 0x00, 0x04,
                                             188,  184,  21,   108};
            memcpy(bytes + sizeof INFO_CERN_QUERY, answer, sizeof answer);
        }
        ISO_CHECK(dns_parse_message(bytes, sizeof bytes, &p).kind == DNS_OK);
        ISO_CHECK(dns_message_is_success(&p));
        ISO_CHECK(p.n_answers == 1);
        ISO_CHECK(name_is(&p.answers[0].name, "info.cern.ch"));
        ISO_CHECK(p.answers[0].ttl == 300);
        got = dns_message_ipv4_answers(&p, v4, 4);
        ISO_CHECK(got == 1 && v4[0][0] == 188 && v4[0][1] == 184 &&
                  v4[0][2] == 21 && v4[0][3] == 108);
        dns_message_free(&p);
    }

    /* ── compressed CNAME target ───────────────────────────────────────────*/
    {
        uint8_t bytes[sizeof INFO_CERN_QUERY + 20];
        DnsMessage p;
        static const uint8_t answer[] = {0xc0, 0x0c, 0x00, 0x05, 0x00, 0x01,
                                         0x00, 0x00, 0x00, 0x3c, 0x00, 0x08,
                                         0x05, 'a',  'l',  'i',  'a',  's',
                                         0xc0, 0x11};
        memcpy(bytes, INFO_CERN_QUERY, sizeof INFO_CERN_QUERY);
        bytes[2] = 0x81;
        bytes[3] = 0x80;
        bytes[6] = 0x00;
        bytes[7] = 0x01;
        memcpy(bytes + sizeof INFO_CERN_QUERY, answer, sizeof answer);
        ISO_CHECK(dns_parse_message(bytes, sizeof bytes, &p).kind == DNS_OK);
        ISO_CHECK(p.answers[0].rrtype.kind == DNS_TYPE_CNAME);
        ISO_CHECK(p.answers[0].data.kind == DNS_RDATA_CNAME);
        ISO_CHECK(name_is(&p.answers[0].data.name, "alias.cern.ch"));
        dns_message_free(&p);
    }

    /* ── AAAA round-trip through serialize ─────────────────────────────────*/
    {
        DnsName name;
        DnsMessage m, p;
        uint8_t *bytes;
        size_t n;
        static const uint8_t v6[16] = {0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0,
                                       0,    0,    0,    0,    0, 0, 0, 1};
        memset(&m, 0, sizeof m);
        ISO_CHECK(dns_name_from_ascii("example.com", &name).kind == DNS_OK);
        m.header.flags = dns_flags_query();
        m.header.flags.is_response = 1;
        m.header.flags.recursion_available = 1;
        m.answers = (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_answers = 1;
        m.answers[0].name = name;
        m.answers[0].rrtype = dns_record_type_known(DNS_TYPE_AAAA);
        m.answers[0].class_.kind = DNS_CLASS_IN;
        m.answers[0].ttl = 10;
        m.answers[0].data.kind = DNS_RDATA_AAAA;
        memcpy(m.answers[0].data.aaaa, v6, 16);
        ISO_CHECK(dns_serialize_message(&m, &bytes, &n).kind == DNS_OK);
        ISO_CHECK(dns_parse_message(bytes, n, &p).kind == DNS_OK);
        {
            uint8_t out6[1][16];
            ISO_CHECK(dns_message_ipv6_answers(&p, out6, 1) == 1);
            ISO_CHECK(memcmp(out6[0], v6, 16) == 0);
        }
        ISO_CHECK(p.answers[0].ttl == 10);
        free(bytes);
        dns_message_free(&m);
        dns_message_free(&p);
    }

    /* ── unknown record data preserved through a round-trip ────────────────*/
    {
        DnsName name;
        DnsMessage m, p;
        uint8_t *bytes;
        size_t n;
        memset(&m, 0, sizeof m);
        ISO_CHECK(dns_name_from_ascii("example.com", &name).kind == DNS_OK);
        m.header.flags = dns_flags_query();
        m.answers = (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_answers = 1;
        m.answers[0].name = name;
        m.answers[0].rrtype.kind = DNS_TYPE_UNKNOWN;
        m.answers[0].rrtype.value = 65;
        m.answers[0].class_.kind = DNS_CLASS_IN;
        m.answers[0].ttl = 1;
        m.answers[0].data.kind = DNS_RDATA_RAW;
        m.answers[0].data.raw = (uint8_t *)malloc(3);
        m.answers[0].data.raw[0] = 1;
        m.answers[0].data.raw[1] = 2;
        m.answers[0].data.raw[2] = 3;
        m.answers[0].data.raw_len = 3;
        ISO_CHECK(dns_serialize_message(&m, &bytes, &n).kind == DNS_OK);
        ISO_CHECK(dns_parse_message(bytes, n, &p).kind == DNS_OK);
        ISO_CHECK(p.answers[0].rrtype.kind == DNS_TYPE_UNKNOWN &&
                  p.answers[0].rrtype.value == 65);
        ISO_CHECK(p.answers[0].data.kind == DNS_RDATA_RAW &&
                  p.answers[0].data.raw_len == 3 &&
                  p.answers[0].data.raw[0] == 1 &&
                  p.answers[0].data.raw[2] == 3);
        free(bytes);
        dns_message_free(&m);
        dns_message_free(&p);
    }

    /* ── PTR + SRV (DNS-SD) round-trip ─────────────────────────────────────*/
    {
        DnsName svc, inst, tgt, inst2;
        DnsMessage m, p;
        uint8_t *bytes;
        size_t n;
        memset(&m, 0, sizeof m);
        ISO_CHECK(dns_name_from_ascii("_hue._tcp.local", &svc).kind == DNS_OK);
        ISO_CHECK(dns_name_from_ascii("bridge-1._hue._tcp.local", &inst).kind ==
                  DNS_OK);
        ISO_CHECK(dns_name_from_ascii("bridge-1._hue._tcp.local", &inst2).kind ==
                  DNS_OK);
        ISO_CHECK(dns_name_from_ascii("bridge-1.local", &tgt).kind == DNS_OK);
        m.header.flags = dns_flags_query();
        m.header.flags.is_response = 1;
        m.header.flags.authoritative_answer = 1;
        m.header.flags.recursion_desired = 0;
        m.answers = (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_answers = 1;
        m.answers[0].name = svc;
        m.answers[0].rrtype = dns_record_type_known(DNS_TYPE_PTR);
        m.answers[0].class_.kind = DNS_CLASS_IN;
        m.answers[0].ttl = 120;
        m.answers[0].data.kind = DNS_RDATA_PTR;
        m.answers[0].data.name = inst;
        m.additionals =
            (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_additionals = 1;
        m.additionals[0].name = inst2;
        m.additionals[0].rrtype = dns_record_type_known(DNS_TYPE_SRV);
        m.additionals[0].class_.kind = DNS_CLASS_IN;
        m.additionals[0].ttl = 120;
        m.additionals[0].data.kind = DNS_RDATA_SRV;
        m.additionals[0].data.srv.priority = 0;
        m.additionals[0].data.srv.weight = 0;
        m.additionals[0].data.srv.port = 443;
        m.additionals[0].data.srv.target = tgt;

        ISO_CHECK(dns_serialize_message(&m, &bytes, &n).kind == DNS_OK);
        ISO_CHECK(dns_parse_message(bytes, n, &p).kind == DNS_OK);
        ISO_CHECK(p.answers[0].data.kind == DNS_RDATA_PTR);
        ISO_CHECK(name_is(&p.answers[0].data.name, "bridge-1._hue._tcp.local"));
        ISO_CHECK(p.additionals[0].data.kind == DNS_RDATA_SRV);
        ISO_CHECK(p.additionals[0].data.srv.port == 443);
        ISO_CHECK(name_is(&p.additionals[0].data.srv.target, "bridge-1.local"));
        free(bytes);
        dns_message_free(&m);
        dns_message_free(&p);
    }

    /* ── first_answer_of_type ──────────────────────────────────────────────*/
    {
        DnsName name;
        DnsMessage m;
        memset(&m, 0, sizeof m);
        ISO_CHECK(dns_name_from_ascii("example.com", &name).kind == DNS_OK);
        m.header.flags = dns_flags_query();
        m.answers = (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_answers = 1;
        m.answers[0].name = name;
        m.answers[0].rrtype = dns_record_type_known(DNS_TYPE_A);
        m.answers[0].data.kind = DNS_RDATA_A;
        ISO_CHECK(dns_message_first_answer_of_type(
                      &m, dns_record_type_known(DNS_TYPE_A)) != NULL);
        ISO_CHECK(dns_message_first_answer_of_type(
                      &m, dns_record_type_known(DNS_TYPE_AAAA)) == NULL);
        dns_message_free(&m);
    }

    /* ── unknown opcode + response code ────────────────────────────────────*/
    {
        static const uint8_t bytes[] = {0x00, 0x01, 0xf0, 0x0f, 0x00, 0x00,
                                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        DnsMessage p;
        ISO_CHECK(dns_parse_message(bytes, sizeof bytes, &p).kind == DNS_OK);
        ISO_CHECK(p.header.flags.opcode.kind == DNS_OPCODE_UNKNOWN &&
                  p.header.flags.opcode.value == 14);
        ISO_CHECK(p.header.flags.response_code.kind == DNS_RCODE_UNKNOWN &&
                  p.header.flags.response_code.value == 15);
        dns_message_free(&p);
    }

    /* ── error cases ───────────────────────────────────────────────────────*/
    {
        static const uint8_t hdr11[11] = {0};
        expect_parse_err(hdr11, 11, DNS_ERR_TRUNCATED_HEADER);
    }
    /* truncated inside the question */
    expect_parse_err(INFO_CERN_QUERY, sizeof INFO_CERN_QUERY - 1,
                     DNS_ERR_UNEXPECTED_EOF);
    /* pointer loop */
    {
        static const uint8_t loop[] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                       0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                       0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01};
        expect_parse_err(loop, sizeof loop, DNS_ERR_POINTER_LOOP);
    }
    /* pointer out of bounds (offset 255) */
    {
        static const uint8_t oob[] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                      0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                      0xc0, 0xff, 0x00, 0x01, 0x00, 0x01};
        DnsMessage m;
        DnsError e = dns_parse_message(oob, sizeof oob, &m);
        ISO_CHECK(e.kind == DNS_ERR_POINTER_OUT_OF_BOUNDS && e.detail == 255);
        if (e.kind == DNS_OK) dns_message_free(&m);
    }
    /* pointer missing its second byte */
    {
        static const uint8_t partial[] = {0x00, 0x01, 0x01, 0x00, 0x00,
                                          0x01, 0x00, 0x00, 0x00, 0x00,
                                          0x00, 0x00, 0xc0};
        expect_parse_err(partial, sizeof partial, DNS_ERR_UNEXPECTED_EOF);
    }
    /* non-ASCII wire label */
    {
        static const uint8_t nonascii[] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                           0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                           0x01, 0xff, 0x00, 0x00, 0x01, 0x00,
                                           0x01};
        expect_parse_err(nonascii, sizeof nonascii, DNS_ERR_NON_ASCII_LABEL);
    }
    /* reserved label prefix (0x40) */
    {
        static const uint8_t reserved[] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                           0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                           0x40, 0x00, 0x01, 0x00, 0x01};
        expect_parse_err(reserved, sizeof reserved, DNS_ERR_UNSUPPORTED);
    }
    /* wire name longer than 255 (4 x 63-byte labels) */
    {
        uint8_t big[12 + 4 * 64 + 5];
        size_t pos = 0, r;
        static const uint8_t hdr[12] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        memcpy(big, hdr, 12);
        pos = 12;
        for (r = 0; r < 4; r++) {
            size_t k;
            big[pos++] = 63;
            for (k = 0; k < 63; k++) big[pos++] = 'a';
        }
        big[pos++] = 0x00;
        big[pos++] = 0x00;
        big[pos++] = 0x01;
        big[pos++] = 0x00;
        big[pos++] = 0x01;
        expect_parse_err(big, pos, DNS_ERR_NAME_TOO_LONG);
    }

    /* ── excessive pointer chain trips the hop cap ─────────────────────────*/
    {
        /* Header + a chain of pointers each aiming two bytes further, which
         * eventually revisits and/or exceeds the hop cap. */
        uint8_t chain[12 + (128 + 2) * 2];
        size_t pos = 0, hop;
        static const uint8_t hdr[12] = {0x00, 0x01, 0x01, 0x00, 0x00, 0x01,
                                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00};
        memcpy(chain, hdr, 12);
        pos = 12;
        for (hop = 0; hop <= 128; hop++) {
            size_t target = 12 + ((hop + 1) * 2);
            chain[pos++] = (uint8_t)(0xc0 | ((target >> 8) & 0x3f));
            chain[pos++] = (uint8_t)(target & 0xff);
        }
        expect_parse_err(chain, pos, DNS_ERR_POINTER_LOOP);
    }

    /* ── serialize-side guards ─────────────────────────────────────────────*/
    {
        /* too many questions: guard fires before touching the (NULL) array */
        DnsMessage m;
        uint8_t *bytes = NULL;
        size_t n = 0;
        memset(&m, 0, sizeof m);
        m.header.flags = dns_flags_query();
        m.n_questions = 0x10000; /* > u16::MAX */
        ISO_CHECK(dns_serialize_message(&m, &bytes, &n).kind ==
                  DNS_ERR_INVALID_SECTION_COUNT);
        ISO_CHECK(bytes == NULL);
    }
    {
        /* rdata larger than the 16-bit length field */
        DnsName name;
        DnsMessage m;
        uint8_t *bytes = NULL;
        size_t n = 0;
        memset(&m, 0, sizeof m);
        ISO_CHECK(dns_name_from_ascii(".", &name).kind == DNS_OK);
        m.header.flags = dns_flags_query();
        m.answers = (DnsResourceRecord *)calloc(1, sizeof(DnsResourceRecord));
        m.n_answers = 1;
        m.answers[0].name = name;
        m.answers[0].rrtype.kind = DNS_TYPE_UNKNOWN;
        m.answers[0].rrtype.value = 65000;
        m.answers[0].class_.kind = DNS_CLASS_IN;
        m.answers[0].data.kind = DNS_RDATA_RAW;
        m.answers[0].data.raw_len = 0x10000; /* 65536 > 65535 */
        m.answers[0].data.raw = (uint8_t *)calloc(0x10000, 1);
        ISO_CHECK(dns_serialize_message(&m, &bytes, &n).kind ==
                  DNS_ERR_UNSUPPORTED);
        ISO_CHECK(bytes == NULL);
        dns_message_free(&m);
    }

    /* ── question record types round-trip ──────────────────────────────────*/
    {
        DnsRecordTypeKind kinds[] = {DNS_TYPE_NS,   DNS_TYPE_CNAME,
                                     DNS_TYPE_SOA,  DNS_TYPE_PTR,
                                     DNS_TYPE_MX,   DNS_TYPE_TXT,
                                     DNS_TYPE_AAAA, DNS_TYPE_SRV};
        size_t i;
        for (i = 0; i < sizeof kinds / sizeof kinds[0]; i++) {
            DnsName name;
            DnsMessage q, p;
            uint8_t *bytes;
            size_t n;
            ISO_CHECK(dns_name_from_ascii("example.com", &name).kind == DNS_OK);
            ISO_CHECK(dns_build_query(5, name, dns_record_type_known(kinds[i]),
                                      &q)
                          .kind == DNS_OK);
            ISO_CHECK(dns_serialize_message(&q, &bytes, &n).kind == DNS_OK);
            ISO_CHECK(dns_parse_message(bytes, n, &p).kind == DNS_OK);
            ISO_CHECK(p.questions[0].qtype.kind == kinds[i]);
            free(bytes);
            dns_message_free(&q);
            dns_message_free(&p);
        }
    }

    return ISO_TEST_RESULT();
}
