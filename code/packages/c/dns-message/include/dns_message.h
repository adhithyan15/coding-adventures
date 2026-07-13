/*
 * dns_message.h — the DNS wire-format layer, pure ISO C17.
 * =======================================================
 *
 * A faithful port of the Rust `dns-message` crate: it turns structured DNS
 * questions and answers into bytes and back. It does NOT open sockets, retry,
 * cache, or choose a nameserver — that keeps it usable over UDP, TCP, a
 * simulated stack, or fixtures.
 *
 * ── Wire format (RFC 1035) ─────────────────────────────────────────────────
 * A message is a 12-byte header (id, a packed flag word, and four section
 * counts) followed by the question, answer, authority, and additional sections.
 * Names are sequences of length-prefixed labels ending in a zero byte; a label
 * length byte whose top two bits are 11 is a *compression pointer* to an offset
 * earlier in the message. This decoder follows pointers under a 128-hop cap
 * (and a 255-byte encoded-name cap) so a malicious message can't loop it
 * forever.
 *
 * ── Ownership ──────────────────────────────────────────────────────────────
 * Structured values own their heap (label strings, record arrays, raw rdata).
 * Every `*_free` releases a value's contents; `parse_dns_message` fills an owned
 * `DnsMessage` you release with `dns_message_free`. Encoding never fails except
 * on a structurally impossible message (too many records, over-long rdata).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef DNS_MESSAGE_H
#define DNS_MESSAGE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Errors ────────────────────────────────────────────────────────────────*/

typedef enum {
    DNS_OK = 0,
    DNS_ERR_TRUNCATED_HEADER,
    DNS_ERR_UNEXPECTED_EOF,
    DNS_ERR_LABEL_TOO_LONG,       /* detail = the offending length */
    DNS_ERR_NAME_TOO_LONG,
    DNS_ERR_POINTER_OUT_OF_BOUNDS, /* detail = the offending offset */
    DNS_ERR_POINTER_LOOP,
    DNS_ERR_NON_ASCII_LABEL,
    DNS_ERR_INVALID_SECTION_COUNT,
    DNS_ERR_UNSUPPORTED,           /* message = a static reason string */
    DNS_ERR_OUT_OF_MEMORY
} DnsErrorKind;

/* A structural encode/decode error. `message` (for DNS_ERR_UNSUPPORTED) borrows
 * a static string; `detail` carries a length/offset for the parametric kinds. */
typedef struct {
    DnsErrorKind kind;
    const char *message;
    size_t detail;
} DnsError;

/* A human-readable label for an error kind. */
const char *dns_error_kind_str(DnsErrorKind kind);

/* ── DnsName ───────────────────────────────────────────────────────────────*/

/* A domain name as human-readable ASCII labels (no trailing root label). The
 * root name `.` is the empty label list. */
typedef struct {
    char **labels; /* owned array of owned NUL-terminated ASCII labels */
    size_t n_labels;
} DnsName;

/* Parse a dotted ASCII name (a trailing '.' and "." both mean the root).
 * Returns DNS_OK and fills `*out`, or an error. */
DnsError dns_name_from_ascii(const char *input, DnsName *out);
/* Release a name's labels and zero it. */
void dns_name_free(DnsName *name);
/* Deep-copy a name into `*out` (DNS_OK, or DNS_ERR_OUT_OF_MEMORY). */
DnsError dns_name_clone(const DnsName *name, DnsName *out);
/* True when this is the root name. */
int dns_name_is_root(const DnsName *name);
/* Render as a dotted string ("." for root) into an owned buffer the caller
 * frees; NULL on OOM. */
char *dns_name_to_string(const DnsName *name);
/* Structural equality. */
int dns_name_equal(const DnsName *a, const DnsName *b);

/* ── Header flags / enums ──────────────────────────────────────────────────*/

typedef enum { DNS_OPCODE_QUERY, DNS_OPCODE_UNKNOWN } DnsOpcodeKind;
typedef struct {
    DnsOpcodeKind kind;
    uint8_t value; /* for DNS_OPCODE_UNKNOWN */
} DnsOpcode;

typedef enum {
    DNS_RCODE_NO_ERROR,
    DNS_RCODE_FORMAT_ERROR,
    DNS_RCODE_SERVER_FAILURE,
    DNS_RCODE_NAME_ERROR,
    DNS_RCODE_NOT_IMPLEMENTED,
    DNS_RCODE_REFUSED,
    DNS_RCODE_UNKNOWN
} DnsResponseCodeKind;
typedef struct {
    DnsResponseCodeKind kind;
    uint8_t value; /* for DNS_RCODE_UNKNOWN */
} DnsResponseCode;

typedef struct {
    int is_response;
    DnsOpcode opcode;
    int authoritative_answer;
    int truncated;
    int recursion_desired;
    int recursion_available;
    DnsResponseCode response_code;
} DnsFlags;

/* A standard recursive query's flags (RD set, everything else default). */
DnsFlags dns_flags_query(void);

typedef struct {
    uint16_t id;
    DnsFlags flags;
    uint16_t question_count;
    uint16_t answer_count;
    uint16_t authority_count;
    uint16_t additional_count;
} DnsHeader;

/* ── Record type / class ───────────────────────────────────────────────────*/

typedef enum {
    DNS_TYPE_A,
    DNS_TYPE_NS,
    DNS_TYPE_CNAME,
    DNS_TYPE_SOA,
    DNS_TYPE_PTR,
    DNS_TYPE_MX,
    DNS_TYPE_TXT,
    DNS_TYPE_AAAA,
    DNS_TYPE_SRV,
    DNS_TYPE_UNKNOWN
} DnsRecordTypeKind;
typedef struct {
    DnsRecordTypeKind kind;
    uint16_t value; /* for DNS_TYPE_UNKNOWN */
} DnsRecordType;

DnsRecordType dns_record_type_known(DnsRecordTypeKind kind); /* not UNKNOWN */
uint16_t dns_record_type_to_u16(DnsRecordType t);
int dns_record_type_equal(DnsRecordType a, DnsRecordType b);

typedef enum { DNS_CLASS_IN, DNS_CLASS_UNKNOWN } DnsClassKind;
typedef struct {
    DnsClassKind kind;
    uint16_t value; /* for DNS_CLASS_UNKNOWN */
} DnsClass;

/* ── Questions / records ───────────────────────────────────────────────────*/

typedef struct {
    DnsName name;
    DnsRecordType qtype;
    DnsClass qclass;
} DnsQuestion;

typedef struct {
    uint16_t priority;
    uint16_t weight;
    uint16_t port;
    DnsName target;
} DnsSrvRecord;

typedef enum {
    DNS_RDATA_A,
    DNS_RDATA_AAAA,
    DNS_RDATA_CNAME,
    DNS_RDATA_PTR,
    DNS_RDATA_SRV,
    DNS_RDATA_RAW
} DnsRecordDataKind;

/* The interpreted payload of a resource record (a tagged union). */
typedef struct {
    DnsRecordDataKind kind;
    uint8_t a[4];        /* DNS_RDATA_A */
    uint8_t aaaa[16];    /* DNS_RDATA_AAAA */
    DnsName name;        /* DNS_RDATA_CNAME / DNS_RDATA_PTR */
    DnsSrvRecord srv;    /* DNS_RDATA_SRV */
    uint8_t *raw;        /* DNS_RDATA_RAW (owned) */
    size_t raw_len;
} DnsRecordData;

typedef struct {
    DnsName name;
    DnsRecordType rrtype;
    DnsClass class_;
    uint32_t ttl;
    DnsRecordData data;
} DnsResourceRecord;

typedef struct {
    DnsHeader header;
    DnsQuestion *questions;
    size_t n_questions;
    DnsResourceRecord *answers;
    size_t n_answers;
    DnsResourceRecord *authorities;
    size_t n_authorities;
    DnsResourceRecord *additionals;
    size_t n_additionals;
} DnsMessage;

/* Release every owned member of a message and zero it. */
void dns_message_free(DnsMessage *message);

/* ── Top-level codec ───────────────────────────────────────────────────────*/

/* Build a standard recursive single-question query. Takes ownership of `name`
 * (moves it into the message; do not free `name` afterwards). */
DnsError dns_build_query(uint16_t id, DnsName name, DnsRecordType qtype,
                         DnsMessage *out);

/* Parse raw wire bytes into `*out` (owned; release with dns_message_free). */
DnsError dns_parse_message(const uint8_t *input, size_t len, DnsMessage *out);

/* Serialize `message` into an owned byte buffer (`*out_bytes` / `*out_len`; the
 * caller frees `*out_bytes`). V1 emits uncompressed names. */
DnsError dns_serialize_message(const DnsMessage *message, uint8_t **out_bytes,
                               size_t *out_len);

/* ── Message accessors ─────────────────────────────────────────────────────*/

/* True for a successful response (is_response set, response code NoError). */
int dns_message_is_success(const DnsMessage *message);
/* The first answer of the given type, or NULL. */
const DnsResourceRecord *dns_message_first_answer_of_type(
    const DnsMessage *message, DnsRecordType qtype);
/* Copy every IPv4 (A) answer address into `out` (each 4 bytes); returns the
 * count. If `out` is NULL just counts. `cap` limits how many are written. */
size_t dns_message_ipv4_answers(const DnsMessage *message, uint8_t (*out)[4],
                                size_t cap);
/* Copy every IPv6 (AAAA) answer address into `out` (each 16 bytes). */
size_t dns_message_ipv6_answers(const DnsMessage *message, uint8_t (*out)[16],
                                size_t cap);

#ifdef __cplusplus
}
#endif

#endif /* DNS_MESSAGE_H */
