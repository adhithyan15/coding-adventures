/*
 * ieee802154_core.c — IEEE 802.15.4 MAC frame primitives, pure ISO C17.
 * =====================================================================
 *
 * See ieee802154_core.h. A faithful port of the Rust `ieee802154-core` crate.
 * Parsing is driven by a bounds-checked little-endian cursor; encoding appends
 * to a growable byte buffer. All field layouts match the Rust exactly.
 */
#include "ieee802154_core.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* ── Bit-field enums ────────────────────────────────────────────────────────*/
size_t ie_address_mode_encoded_len(IE_AddressMode mode) {
    switch (mode) {
    case IE_ADDR_SHORT:
        return 2;
    case IE_ADDR_EXTENDED:
        return 8;
    case IE_ADDR_NONE:
    case IE_ADDR_RESERVED:
        return 0;
    }
    return 0;
}

IE_FrameControl ie_frame_control_parse(uint16_t raw) {
    IE_FrameControl fc;
    fc.frame_type = (IE_FrameType)(raw & 0x7);
    fc.security_enabled = (raw & (1u << 3)) != 0;
    fc.frame_pending = (raw & (1u << 4)) != 0;
    fc.ack_request = (raw & (1u << 5)) != 0;
    fc.pan_id_compression = (raw & (1u << 6)) != 0;
    fc.sequence_number_suppression = (raw & (1u << 8)) != 0;
    fc.information_elements_present = (raw & (1u << 9)) != 0;
    fc.destination_address_mode = (IE_AddressMode)((raw >> 10) & 0x3);
    fc.frame_version = (IE_FrameVersion)((raw >> 12) & 0x3);
    fc.source_address_mode = (IE_AddressMode)((raw >> 14) & 0x3);
    return fc;
}

uint16_t ie_frame_control_encode(IE_FrameControl fc) {
    uint16_t raw = (uint16_t)fc.frame_type;
    raw |= (uint16_t)((fc.security_enabled ? 1u : 0u) << 3);
    raw |= (uint16_t)((fc.frame_pending ? 1u : 0u) << 4);
    raw |= (uint16_t)((fc.ack_request ? 1u : 0u) << 5);
    raw |= (uint16_t)((fc.pan_id_compression ? 1u : 0u) << 6);
    raw |= (uint16_t)((fc.sequence_number_suppression ? 1u : 0u) << 8);
    raw |= (uint16_t)((fc.information_elements_present ? 1u : 0u) << 9);
    raw |= (uint16_t)((unsigned)fc.destination_address_mode << 10);
    raw |= (uint16_t)((unsigned)fc.frame_version << 12);
    raw |= (uint16_t)((unsigned)fc.source_address_mode << 14);
    return raw;
}

int ie_security_level_encrypts(IE_SecurityLevel level) {
    return level == IE_SEC_ENC || level == IE_SEC_ENC_MIC32 ||
           level == IE_SEC_ENC_MIC64 || level == IE_SEC_ENC_MIC128;
}
size_t ie_security_level_mic_len(IE_SecurityLevel level) {
    switch (level) {
    case IE_SEC_NONE:
    case IE_SEC_ENC:
        return 0;
    case IE_SEC_MIC32:
    case IE_SEC_ENC_MIC32:
        return 4;
    case IE_SEC_MIC64:
    case IE_SEC_ENC_MIC64:
        return 8;
    case IE_SEC_MIC128:
    case IE_SEC_ENC_MIC128:
        return 16;
    }
    return 0;
}

IE_SecurityControl ie_security_control_parse(uint8_t raw) {
    IE_SecurityControl sc;
    sc.security_level = (IE_SecurityLevel)(raw & 0x7);
    sc.key_identifier_mode = (IE_KeyIdMode)((raw >> 3) & 0x3);
    sc.frame_counter_suppression = (raw & (1u << 5)) != 0;
    sc.frame_counter_size_5 = (raw & (1u << 6)) != 0;
    return sc;
}
uint8_t ie_security_control_encode(IE_SecurityControl sc) {
    uint8_t raw = (uint8_t)sc.security_level;
    raw |= (uint8_t)((unsigned)sc.key_identifier_mode << 3);
    raw |= (uint8_t)((sc.frame_counter_suppression ? 1u : 0u) << 5);
    raw |= (uint8_t)((sc.frame_counter_size_5 ? 1u : 0u) << 6);
    return raw;
}

const char *ie_mac_error_str(IE_MacError e) {
    switch (e) {
    case IE_MAC_OK:
        return "ok";
    case IE_MAC_ERR_TRUNCATED:
        return "truncated IEEE 802.15.4 frame";
    case IE_MAC_ERR_RESERVED_ADDRESS_MODE:
        return "reserved address mode";
    case IE_MAC_ERR_ADDRESS_MODE_MISMATCH:
        return "frame-control address mode does not match address";
    case IE_MAC_ERR_MISSING_SEQUENCE_NUMBER:
        return "missing sequence number";
    case IE_MAC_ERR_MISSING_DESTINATION_PAN_ID:
        return "missing destination PAN id";
    case IE_MAC_ERR_MISSING_SOURCE_PAN_ID:
        return "missing source PAN id";
    case IE_MAC_ERR_MISSING_AUX_SECURITY_HEADER:
        return "missing auxiliary security header";
    case IE_MAC_ERR_UNEXPECTED_AUX_SECURITY_HEADER:
        return "unexpected auxiliary security header";
    case IE_MAC_ERR_MISSING_FRAME_COUNTER:
        return "missing frame counter";
    case IE_MAC_ERR_UNEXPECTED_FRAME_COUNTER:
        return "unexpected frame counter";
    case IE_MAC_ERR_FRAME_COUNTER_SIZE_MISMATCH:
        return "frame counter size mismatch";
    case IE_MAC_ERR_FRAME_COUNTER_OUT_OF_RANGE:
        return "40-bit frame counter is out of range";
    case IE_MAC_ERR_KEY_IDENTIFIER_MODE_MISMATCH:
        return "key identifier mode mismatch";
    }
    return "unknown error";
}

const char *ie_beacon_error_str(IE_BeaconError e) {
    switch (e) {
    case IE_BEACON_OK:
        return "ok";
    case IE_BEACON_ERR_EXPECTED_BEACON_FRAME:
        return "expected an IEEE 802.15.4 beacon frame";
    case IE_BEACON_ERR_MISSING_SOURCE_ADDRESS:
        return "beacon frame is missing coordinator source address";
    case IE_BEACON_ERR_MISSING_PAN_ID:
        return "beacon frame is missing coordinator PAN id";
    case IE_BEACON_ERR_TRUNCATED_FIELD:
        return "truncated IEEE 802.15.4 beacon payload field";
    }
    return "unknown error";
}

/* ── Parsing cursor (bounds-checked, little-endian) ─────────────────────────*/
typedef struct {
    const uint8_t *bytes;
    size_t len;
    size_t off;
} Cursor;

static size_t cur_remaining(const Cursor *c) {
    return c->len > c->off ? c->len - c->off : 0;
}
static int cur_read(Cursor *c, size_t n, const uint8_t **out) {
    if (cur_remaining(c) < n) {
        return 0;
    }
    *out = c->bytes + c->off;
    c->off += n;
    return 1;
}
static int cur_u8(Cursor *c, uint8_t *out) {
    const uint8_t *p;
    if (!cur_read(c, 1, &p)) {
        return 0;
    }
    *out = p[0];
    return 1;
}
static int cur_u16(Cursor *c, uint16_t *out) {
    const uint8_t *p;
    if (!cur_read(c, 2, &p)) {
        return 0;
    }
    *out = (uint16_t)(p[0] | ((uint16_t)p[1] << 8));
    return 1;
}
static int cur_u32(Cursor *c, uint32_t *out) {
    const uint8_t *p;
    if (!cur_read(c, 4, &p)) {
        return 0;
    }
    *out = (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
    return 1;
}
static int cur_u40(Cursor *c, uint64_t *out) {
    const uint8_t *p;
    if (!cur_read(c, 5, &p)) {
        return 0;
    }
    *out = (uint64_t)p[0] | ((uint64_t)p[1] << 8) | ((uint64_t)p[2] << 16) |
           ((uint64_t)p[3] << 24) | ((uint64_t)p[4] << 32);
    return 1;
}
static int cur_u64(Cursor *c, uint64_t *out) {
    const uint8_t *p;
    if (!cur_read(c, 8, &p)) {
        return 0;
    }
    *out = (uint64_t)p[0] | ((uint64_t)p[1] << 8) | ((uint64_t)p[2] << 16) |
           ((uint64_t)p[3] << 24) | ((uint64_t)p[4] << 32) |
           ((uint64_t)p[5] << 40) | ((uint64_t)p[6] << 48) |
           ((uint64_t)p[7] << 56);
    return 1;
}

/* ── Growable output buffer (size_t-overflow-guarded) ───────────────────────*/
typedef struct {
    uint8_t *data;
    size_t len, cap;
    int ok;
} Buf;

static void buf_push(Buf *b, uint8_t v) {
    if (!b->ok) {
        return;
    }
    if (b->len == b->cap) {
        size_t nc = b->cap ? b->cap : 32;
        uint8_t *nd;
        if (nc > (size_t)-1 / 2) {
            b->ok = 0;
            return;
        }
        nc *= 2;
        nd = (uint8_t *)realloc(b->data, nc);
        if (!nd) {
            b->ok = 0;
            return;
        }
        b->data = nd;
        b->cap = nc;
    }
    b->data[b->len++] = v;
}
static void buf_extend(Buf *b, const uint8_t *src, size_t n) {
    size_t i;
    for (i = 0; i < n; i++) {
        buf_push(b, src[i]);
    }
}
static void buf_u16(Buf *b, uint16_t v) {
    buf_push(b, (uint8_t)v);
    buf_push(b, (uint8_t)(v >> 8));
}

/* ── MAC frame parse ────────────────────────────────────────────────────────*/
static int read_address(Cursor *c, IE_AddressMode mode, IE_Address *out,
                        IE_MacError *err) {
    switch (mode) {
    case IE_ADDR_NONE:
        return 0; /* no address */
    case IE_ADDR_RESERVED:
        *err = IE_MAC_ERR_RESERVED_ADDRESS_MODE;
        return -1;
    case IE_ADDR_SHORT: {
        uint16_t v;
        if (!cur_u16(c, &v)) {
            *err = IE_MAC_ERR_TRUNCATED;
            return -1;
        }
        out->mode = IE_ADDR_SHORT;
        out->short_addr = v;
        out->extended_addr = 0;
        return 1;
    }
    case IE_ADDR_EXTENDED: {
        uint64_t v;
        if (!cur_u64(c, &v)) {
            *err = IE_MAC_ERR_TRUNCATED;
            return -1;
        }
        out->mode = IE_ADDR_EXTENDED;
        out->extended_addr = v;
        out->short_addr = 0;
        return 1;
    }
    }
    return 0;
}

static IE_MacError parse_aux(Cursor *c, IE_AuxSecurityHeader *out) {
    uint8_t sc_raw;
    if (!cur_u8(c, &sc_raw)) {
        return IE_MAC_ERR_TRUNCATED;
    }
    out->security_control = ie_security_control_parse(sc_raw);
    out->has_frame_counter = 0;
    out->frame_counter.is_40bit = 0;
    out->frame_counter.value = 0;
    if (!out->security_control.frame_counter_suppression) {
        if (out->security_control.frame_counter_size_5) {
            uint64_t v;
            if (!cur_u40(c, &v)) {
                return IE_MAC_ERR_TRUNCATED;
            }
            out->has_frame_counter = 1;
            out->frame_counter.is_40bit = 1;
            out->frame_counter.value = v;
        } else {
            uint32_t v;
            if (!cur_u32(c, &v)) {
                return IE_MAC_ERR_TRUNCATED;
            }
            out->has_frame_counter = 1;
            out->frame_counter.is_40bit = 0;
            out->frame_counter.value = v;
        }
    }
    out->key_identifier.mode = out->security_control.key_identifier_mode;
    memset(out->key_identifier.source, 0, sizeof out->key_identifier.source);
    out->key_identifier.index = 0;
    switch (out->security_control.key_identifier_mode) {
    case IE_KEYID_IMPLICIT:
        break;
    case IE_KEYID_KEY_INDEX:
        if (!cur_u8(c, &out->key_identifier.index)) {
            return IE_MAC_ERR_TRUNCATED;
        }
        break;
    case IE_KEYID_KEY_SOURCE4: {
        const uint8_t *p;
        if (!cur_read(c, 4, &p) || !cur_u8(c, &out->key_identifier.index)) {
            return IE_MAC_ERR_TRUNCATED;
        }
        memcpy(out->key_identifier.source, p, 4);
        break;
    }
    case IE_KEYID_KEY_SOURCE8: {
        const uint8_t *p;
        if (!cur_read(c, 8, &p) || !cur_u8(c, &out->key_identifier.index)) {
            return IE_MAC_ERR_TRUNCATED;
        }
        memcpy(out->key_identifier.source, p, 8);
        break;
    }
    }
    return IE_MAC_OK;
}

IE_MacError ie_mac_frame_parse(const uint8_t *bytes, size_t len, int has_fcs,
                               IE_MacFrame *out) {
    Cursor c;
    uint16_t raw_fcf;
    IE_FrameControl fc;
    IE_MacError err = IE_MAC_OK;
    size_t remaining, payload_len;
    int r;

    memset(out, 0, sizeof *out);
    c.bytes = bytes;
    c.len = len;
    c.off = 0;

    if (!cur_u16(&c, &raw_fcf)) {
        return IE_MAC_ERR_TRUNCATED;
    }
    fc = ie_frame_control_parse(raw_fcf);
    out->frame_control = fc;

    if (fc.destination_address_mode == IE_ADDR_RESERVED ||
        fc.source_address_mode == IE_ADDR_RESERVED) {
        return IE_MAC_ERR_RESERVED_ADDRESS_MODE;
    }

    if (!fc.sequence_number_suppression) {
        if (!cur_u8(&c, &out->sequence_number)) {
            return IE_MAC_ERR_TRUNCATED;
        }
        out->has_sequence_number = 1;
    }

    /* destination PAN id + address */
    if (fc.destination_address_mode != IE_ADDR_NONE) {
        uint16_t pan;
        if (!cur_u16(&c, &pan)) {
            return IE_MAC_ERR_TRUNCATED;
        }
        out->has_destination_pan_id = 1;
        out->destination_pan_id = pan;
        r = read_address(&c, fc.destination_address_mode, &out->destination,
                         &err);
        if (r < 0) {
            return err;
        }
        out->has_destination = (r == 1);
    }

    /* source PAN id */
    if (fc.source_address_mode != IE_ADDR_NONE) {
        if (fc.pan_id_compression && out->has_destination_pan_id) {
            out->has_source_pan_id = 1;
            out->source_pan_id = out->destination_pan_id;
        } else {
            uint16_t pan;
            if (!cur_u16(&c, &pan)) {
                return IE_MAC_ERR_TRUNCATED;
            }
            out->has_source_pan_id = 1;
            out->source_pan_id = pan;
        }
    }

    r = read_address(&c, fc.source_address_mode, &out->source, &err);
    if (r < 0) {
        return err;
    }
    out->has_source = (r == 1);

    if (fc.security_enabled) {
        err = parse_aux(&c, &out->aux_security_header);
        if (err != IE_MAC_OK) {
            return err;
        }
        out->has_aux_security_header = 1;
    }

    remaining = cur_remaining(&c);
    if (has_fcs) {
        if (remaining < 2) {
            return IE_MAC_ERR_TRUNCATED;
        }
        payload_len = remaining - 2;
    } else {
        payload_len = remaining;
    }

    if (payload_len > 0) {
        const uint8_t *p;
        out->payload = (uint8_t *)malloc(payload_len);
        if (!out->payload) {
            return IE_MAC_ERR_TRUNCATED; /* treat OOM as parse failure */
        }
        (void)cur_read(&c, payload_len, &p);
        memcpy(out->payload, p, payload_len);
        out->payload_len = payload_len;
    }

    if (has_fcs) {
        if (!cur_u16(&c, &out->fcs)) {
            free(out->payload);
            out->payload = NULL;
            return IE_MAC_ERR_TRUNCATED;
        }
        out->has_fcs = 1;
    }
    return IE_MAC_OK;
}

/* ── MAC frame encode ───────────────────────────────────────────────────────*/
static IE_AddressMode address_mode_of(int has_addr, const IE_Address *a) {
    return has_addr ? a->mode : IE_ADDR_NONE;
}

static IE_MacError validate_aux(const IE_AuxSecurityHeader *h) {
    if (h->key_identifier.mode != h->security_control.key_identifier_mode) {
        return IE_MAC_ERR_KEY_IDENTIFIER_MODE_MISMATCH;
    }
    if (h->security_control.frame_counter_suppression) {
        return h->has_frame_counter ? IE_MAC_ERR_UNEXPECTED_FRAME_COUNTER
                                    : IE_MAC_OK;
    }
    if (!h->has_frame_counter) {
        return IE_MAC_ERR_MISSING_FRAME_COUNTER;
    }
    if (!h->security_control.frame_counter_size_5) {
        return h->frame_counter.is_40bit ? IE_MAC_ERR_FRAME_COUNTER_SIZE_MISMATCH
                                         : IE_MAC_OK;
    }
    /* size_5 */
    if (!h->frame_counter.is_40bit) {
        return IE_MAC_ERR_FRAME_COUNTER_SIZE_MISMATCH;
    }
    return h->frame_counter.value <= (uint64_t)0xFFFFFFFFFFull
               ? IE_MAC_OK
               : IE_MAC_ERR_FRAME_COUNTER_OUT_OF_RANGE;
}

static IE_MacError validate_modes(const IE_MacFrame *f) {
    IE_AddressMode dm, sm;
    if (f->frame_control.destination_address_mode == IE_ADDR_RESERVED ||
        f->frame_control.source_address_mode == IE_ADDR_RESERVED) {
        return IE_MAC_ERR_RESERVED_ADDRESS_MODE;
    }
    dm = address_mode_of(f->has_destination, &f->destination);
    if (dm != f->frame_control.destination_address_mode) {
        return IE_MAC_ERR_ADDRESS_MODE_MISMATCH;
    }
    sm = address_mode_of(f->has_source, &f->source);
    if (sm != f->frame_control.source_address_mode) {
        return IE_MAC_ERR_ADDRESS_MODE_MISMATCH;
    }
    if (f->frame_control.security_enabled) {
        if (!f->has_aux_security_header) {
            return IE_MAC_ERR_MISSING_AUX_SECURITY_HEADER;
        }
        return validate_aux(&f->aux_security_header);
    }
    if (f->has_aux_security_header) {
        return IE_MAC_ERR_UNEXPECTED_AUX_SECURITY_HEADER;
    }
    return IE_MAC_OK;
}

static void encode_address(const IE_Address *a, Buf *b) {
    if (a->mode == IE_ADDR_SHORT) {
        buf_u16(b, a->short_addr);
    } else {
        int i;
        for (i = 0; i < 8; i++) {
            buf_push(b, (uint8_t)(a->extended_addr >> (i * 8)));
        }
    }
}

static void encode_aux(const IE_AuxSecurityHeader *h, Buf *b) {
    buf_push(b, ie_security_control_encode(h->security_control));
    if (h->has_frame_counter) {
        int n = h->frame_counter.is_40bit ? 5 : 4;
        int i;
        for (i = 0; i < n; i++) {
            buf_push(b, (uint8_t)(h->frame_counter.value >> (i * 8)));
        }
    }
    switch (h->key_identifier.mode) {
    case IE_KEYID_IMPLICIT:
        break;
    case IE_KEYID_KEY_INDEX:
        buf_push(b, h->key_identifier.index);
        break;
    case IE_KEYID_KEY_SOURCE4:
        buf_extend(b, h->key_identifier.source, 4);
        buf_push(b, h->key_identifier.index);
        break;
    case IE_KEYID_KEY_SOURCE8:
        buf_extend(b, h->key_identifier.source, 8);
        buf_push(b, h->key_identifier.index);
        break;
    }
}

IE_MacError ie_mac_frame_encode(const IE_MacFrame *f, uint8_t **out_bytes,
                                size_t *out_len) {
    Buf b;
    IE_MacError err = validate_modes(f);
    if (err != IE_MAC_OK) {
        return err;
    }
    b.data = NULL;
    b.len = 0;
    b.cap = 0;
    b.ok = 1;

    buf_u16(&b, ie_frame_control_encode(f->frame_control));

    if (!f->frame_control.sequence_number_suppression) {
        if (!f->has_sequence_number) {
            free(b.data);
            return IE_MAC_ERR_MISSING_SEQUENCE_NUMBER;
        }
        buf_push(&b, f->sequence_number);
    }

    if (f->has_destination) {
        if (!f->has_destination_pan_id) {
            free(b.data);
            return IE_MAC_ERR_MISSING_DESTINATION_PAN_ID;
        }
        buf_u16(&b, f->destination_pan_id);
        encode_address(&f->destination, &b);
    }

    if (f->has_source) {
        if (!f->frame_control.pan_id_compression ||
            !f->has_destination_pan_id) {
            if (!f->has_source_pan_id) {
                free(b.data);
                return IE_MAC_ERR_MISSING_SOURCE_PAN_ID;
            }
            buf_u16(&b, f->source_pan_id);
        }
        encode_address(&f->source, &b);
    }

    if (f->frame_control.security_enabled) {
        /* validate_modes already ensured the header is present + consistent. */
        encode_aux(&f->aux_security_header, &b);
    }

    buf_extend(&b, f->payload, f->payload_len);

    if (f->has_fcs) {
        buf_u16(&b, f->fcs);
    }

    if (!b.ok) {
        free(b.data);
        return IE_MAC_ERR_TRUNCATED; /* OOM */
    }
    *out_bytes = b.data;
    *out_len = b.len;
    return IE_MAC_OK;
}

void ie_mac_frame_summary(const IE_MacFrame *f, IE_MacFrameSummary *s) {
    s->frame_type = f->frame_control.frame_type;
    s->frame_version = f->frame_control.frame_version;
    s->destination_address_mode = f->frame_control.destination_address_mode;
    s->source_address_mode = f->frame_control.source_address_mode;
    s->security_enabled = f->frame_control.security_enabled;
    s->has_auxiliary_security_header = f->has_aux_security_header;
    s->ack_request = f->frame_control.ack_request;
    s->frame_pending = f->frame_control.frame_pending;
    s->pan_id_compression = f->frame_control.pan_id_compression;
    s->sequence_number_suppressed = f->frame_control.sequence_number_suppression;
    s->information_elements_present = f->frame_control.information_elements_present;
    s->has_sequence_number = f->has_sequence_number;
    s->has_destination_pan_id = f->has_destination_pan_id;
    s->has_source_pan_id = f->has_source_pan_id;
    s->has_destination = f->has_destination;
    s->has_source = f->has_source;
    s->payload_len = f->payload_len;
    s->has_fcs = f->has_fcs;
}

void ie_mac_frame_free(IE_MacFrame *f) {
    if (f) {
        free(f->payload);
        f->payload = NULL;
        f->payload_len = 0;
    }
}

int ie_mac_summary_has_payload(const IE_MacFrameSummary *s) {
    return s->payload_len > 0;
}
int ie_mac_summary_has_addressing(const IE_MacFrameSummary *s) {
    return s->has_destination || s->has_source;
}

/* ── Superframe accessors ───────────────────────────────────────────────────*/
uint8_t ie_superframe_beacon_order(uint16_t raw) { return (uint8_t)(raw & 0xF); }
uint8_t ie_superframe_order(uint16_t raw) {
    return (uint8_t)((raw >> 4) & 0xF);
}
uint8_t ie_superframe_final_cap_slot(uint16_t raw) {
    return (uint8_t)((raw >> 8) & 0xF);
}
int ie_superframe_battery_life_extension(uint16_t raw) {
    return (raw & (1u << 12)) != 0;
}
int ie_superframe_pan_coordinator(uint16_t raw) {
    return (raw & (1u << 14)) != 0;
}
int ie_superframe_association_permit(uint16_t raw) {
    return (raw & (1u << 15)) != 0;
}

/* ── Beacon payload parse ───────────────────────────────────────────────────*/
IE_BeaconError ie_beacon_payload_parse(const uint8_t *bytes, size_t len,
                                       IE_BeaconPayload *out) {
    Cursor c;
    uint16_t superframe;
    uint8_t gts_spec, pending_spec;
    uint8_t descriptor_count, short_count, extended_count;
    size_t i, rest;

    memset(out, 0, sizeof *out);
    c.bytes = bytes;
    c.len = len;
    c.off = 0;

    if (!cur_u16(&c, &superframe)) {
        return IE_BEACON_ERR_TRUNCATED_FIELD;
    }
    out->superframe_raw = superframe;

    if (!cur_u8(&c, &gts_spec)) {
        return IE_BEACON_ERR_TRUNCATED_FIELD;
    }
    descriptor_count = gts_spec & 0x07;
    out->gts_descriptor_count = descriptor_count;
    out->gts_permit = (gts_spec & 0x80) != 0;
    if (descriptor_count != 0) {
        if (!cur_u8(&c, &out->gts_directions)) {
            return IE_BEACON_ERR_TRUNCATED_FIELD;
        }
        out->gts_has_directions = 1;
    }
    for (i = 0; i < descriptor_count; i++) {
        uint16_t addr;
        uint8_t slot_len;
        if (!cur_u16(&c, &addr) || !cur_u8(&c, &slot_len)) {
            return IE_BEACON_ERR_TRUNCATED_FIELD;
        }
        out->gts_descriptors[i].short_address = addr;
        out->gts_descriptors[i].starting_slot = (uint8_t)(slot_len & 0x0F);
        out->gts_descriptors[i].length = (uint8_t)((slot_len >> 4) & 0x0F);
    }

    if (!cur_u8(&c, &pending_spec)) {
        return IE_BEACON_ERR_TRUNCATED_FIELD;
    }
    short_count = pending_spec & 0x07;
    extended_count = (pending_spec >> 4) & 0x07;
    for (i = 0; i < short_count; i++) {
        if (!cur_u16(&c, &out->short_addresses[i])) {
            return IE_BEACON_ERR_TRUNCATED_FIELD;
        }
    }
    out->short_address_count = short_count;
    for (i = 0; i < extended_count; i++) {
        if (!cur_u64(&c, &out->extended_addresses[i])) {
            return IE_BEACON_ERR_TRUNCATED_FIELD;
        }
    }
    out->extended_address_count = extended_count;

    rest = cur_remaining(&c);
    if (rest > 0) {
        const uint8_t *p;
        out->payload = (uint8_t *)malloc(rest);
        if (!out->payload) {
            return IE_BEACON_ERR_TRUNCATED_FIELD; /* OOM */
        }
        (void)cur_read(&c, rest, &p);
        memcpy(out->payload, p, rest);
        out->payload_len = rest;
    }
    return IE_BEACON_OK;
}

void ie_beacon_payload_free(IE_BeaconPayload *bp) {
    if (bp) {
        free(bp->payload);
        bp->payload = NULL;
        bp->payload_len = 0;
    }
}

/* ── PAN descriptor ─────────────────────────────────────────────────────────*/
IE_BeaconError ie_pan_descriptor_from_beacon_frame(const IE_MacFrame *frame,
                                                   uint8_t channel,
                                                   uint8_t channel_page,
                                                   uint8_t link_quality,
                                                   IE_PanDescriptor *out) {
    IE_BeaconError err;
    memset(out, 0, sizeof *out);
    if (frame->frame_control.frame_type != IE_FRAME_BEACON) {
        return IE_BEACON_ERR_EXPECTED_BEACON_FRAME;
    }
    if (!frame->has_source) {
        return IE_BEACON_ERR_MISSING_SOURCE_ADDRESS;
    }
    if (frame->has_source_pan_id) {
        out->coordinator_pan_id = frame->source_pan_id;
    } else if (frame->has_destination_pan_id) {
        out->coordinator_pan_id = frame->destination_pan_id;
    } else {
        return IE_BEACON_ERR_MISSING_PAN_ID;
    }
    err = ie_beacon_payload_parse(frame->payload, frame->payload_len,
                                  &out->beacon);
    if (err != IE_BEACON_OK) {
        return err;
    }
    out->coordinator_address = frame->source;
    out->channel = channel;
    out->channel_page = channel_page;
    out->link_quality = link_quality;
    return IE_BEACON_OK;
}

int ie_pan_descriptor_association_permitted(const IE_PanDescriptor *pd) {
    return ie_superframe_association_permit(pd->beacon.superframe_raw);
}

void ie_pan_descriptor_free(IE_PanDescriptor *pd) {
    if (pd) {
        ie_beacon_payload_free(&pd->beacon);
    }
}

/* ── PAN scan helpers ───────────────────────────────────────────────────────*/
size_t ie_pan_scan_count_for_channel(const IE_PanDescriptor *d, size_t count,
                                     uint8_t channel) {
    size_t i, n = 0;
    for (i = 0; i < count; i++) {
        if (d[i].channel == channel) {
            n++;
        }
    }
    return n;
}
size_t ie_pan_scan_association_candidate_count(const IE_PanDescriptor *d,
                                               size_t count) {
    size_t i, n = 0;
    for (i = 0; i < count; i++) {
        if (ie_pan_descriptor_association_permitted(&d[i])) {
            n++;
        }
    }
    return n;
}
long ie_pan_scan_best_candidate_index(const IE_PanDescriptor *d, size_t count) {
    size_t i;
    long best = -1;
    uint8_t best_lqi = 0;
    int best_pancoord = 0;
    for (i = 0; i < count; i++) {
        int pancoord;
        if (!ie_pan_descriptor_association_permitted(&d[i])) {
            continue;
        }
        pancoord = ie_superframe_pan_coordinator(d[i].beacon.superframe_raw);
        /* max_by_key on (link_quality, pan_coordinator); on a tie the LAST
         * element wins (Rust semantics), so replace on >=. */
        if (best < 0 || d[i].link_quality > best_lqi ||
            (d[i].link_quality == best_lqi && pancoord >= best_pancoord)) {
            best = (long)i;
            best_lqi = d[i].link_quality;
            best_pancoord = pancoord;
        }
    }
    return best;
}
