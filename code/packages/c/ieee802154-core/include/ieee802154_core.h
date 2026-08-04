/*
 * ieee802154_core.h — IEEE 802.15.4 MAC frame primitives, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `ieee802154-core` crate: a small, dependency-free
 * parser/encoder for IEEE 802.15.4 MAC frames — the byte-level foundation both
 * Zigbee and Thread build on. Covers the frame-control field, addressing, the
 * auxiliary security header, beacon payloads (superframe spec, GTS, pending
 * addresses), and PAN descriptors / scan summaries.
 *
 * Every multi-byte field is little-endian; every read is bounds-checked, so a
 * truncated or hostile frame yields an error, never an out-of-bounds access.
 *
 * Divergences from the Rust (documented): error enums drop the diagnostic
 * `field`/`needed`/`remaining` payloads the Rust variants carry (the variant
 * itself is preserved). Bounded MAC counts (GTS descriptors ≤ 7, pending
 * addresses ≤ 7 each) use fixed arrays; variable payloads are heap-owned.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef IEEE802154_CORE_H
#define IEEE802154_CORE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Frame type — the enum value is the 3-bit frame-control field. */
typedef enum {
    IE_FRAME_BEACON = 0,
    IE_FRAME_DATA = 1,
    IE_FRAME_ACK = 2,
    IE_FRAME_MAC_COMMAND = 3,
    IE_FRAME_RESERVED = 4,
    IE_FRAME_MULTIPURPOSE = 5,
    IE_FRAME_FRAGMENT = 6,
    IE_FRAME_EXTENDED = 7
} IE_FrameType;

/* Address mode — the enum value is the 2-bit field. */
typedef enum {
    IE_ADDR_NONE = 0,
    IE_ADDR_RESERVED = 1,
    IE_ADDR_SHORT = 2,
    IE_ADDR_EXTENDED = 3
} IE_AddressMode;

size_t ie_address_mode_encoded_len(IE_AddressMode mode);

/* Frame version — the enum value is the 2-bit field. */
typedef enum {
    IE_VERSION_2003 = 0,
    IE_VERSION_2006 = 1,
    IE_VERSION_2015 = 2,
    IE_VERSION_RESERVED = 3
} IE_FrameVersion;

/* A device address (short 16-bit or extended 64-bit). */
typedef struct {
    IE_AddressMode mode; /* IE_ADDR_SHORT or IE_ADDR_EXTENDED */
    uint16_t short_addr;
    uint64_t extended_addr;
} IE_Address;

/* The 16-bit frame control field, decoded. */
typedef struct {
    IE_FrameType frame_type;
    int security_enabled;
    int frame_pending;
    int ack_request;
    int pan_id_compression;
    int sequence_number_suppression;
    int information_elements_present;
    IE_AddressMode destination_address_mode;
    IE_FrameVersion frame_version;
    IE_AddressMode source_address_mode;
} IE_FrameControl;

IE_FrameControl ie_frame_control_parse(uint16_t raw);
uint16_t ie_frame_control_encode(IE_FrameControl fc);

/* Security level — enum value is the 3-bit field. */
typedef enum {
    IE_SEC_NONE = 0,
    IE_SEC_MIC32 = 1,
    IE_SEC_MIC64 = 2,
    IE_SEC_MIC128 = 3,
    IE_SEC_ENC = 4,
    IE_SEC_ENC_MIC32 = 5,
    IE_SEC_ENC_MIC64 = 6,
    IE_SEC_ENC_MIC128 = 7
} IE_SecurityLevel;

int ie_security_level_encrypts(IE_SecurityLevel level);
size_t ie_security_level_mic_len(IE_SecurityLevel level);

/* Key identifier mode — enum value is the 2-bit field. */
typedef enum {
    IE_KEYID_IMPLICIT = 0,
    IE_KEYID_KEY_INDEX = 1,
    IE_KEYID_KEY_SOURCE4 = 2,
    IE_KEYID_KEY_SOURCE8 = 3
} IE_KeyIdMode;

typedef struct {
    IE_SecurityLevel security_level;
    IE_KeyIdMode key_identifier_mode;
    int frame_counter_suppression;
    int frame_counter_size_5;
} IE_SecurityControl;

IE_SecurityControl ie_security_control_parse(uint8_t raw);
uint8_t ie_security_control_encode(IE_SecurityControl sc);

/* Frame counter (32- or 40-bit). */
typedef struct {
    int is_40bit; /* 0 = 32-bit counter, 1 = 40-bit counter */
    uint64_t value;
} IE_FrameCounter;

/* Key identifier. `source` holds 4 bytes (KeySource4) or 8 (KeySource8). */
typedef struct {
    IE_KeyIdMode mode;
    uint8_t index;
    uint8_t source[8];
} IE_KeyIdentifier;

typedef struct {
    IE_SecurityControl security_control;
    int has_frame_counter;
    IE_FrameCounter frame_counter;
    IE_KeyIdentifier key_identifier;
} IE_AuxSecurityHeader;

/* Errors from MAC frame parse/encode. IE_MAC_OK (0) means success. */
typedef enum {
    IE_MAC_OK = 0,
    IE_MAC_ERR_TRUNCATED,
    IE_MAC_ERR_RESERVED_ADDRESS_MODE,
    IE_MAC_ERR_ADDRESS_MODE_MISMATCH,
    IE_MAC_ERR_MISSING_SEQUENCE_NUMBER,
    IE_MAC_ERR_MISSING_DESTINATION_PAN_ID,
    IE_MAC_ERR_MISSING_SOURCE_PAN_ID,
    IE_MAC_ERR_MISSING_AUX_SECURITY_HEADER,
    IE_MAC_ERR_UNEXPECTED_AUX_SECURITY_HEADER,
    IE_MAC_ERR_MISSING_FRAME_COUNTER,
    IE_MAC_ERR_UNEXPECTED_FRAME_COUNTER,
    IE_MAC_ERR_FRAME_COUNTER_SIZE_MISMATCH,
    IE_MAC_ERR_FRAME_COUNTER_OUT_OF_RANGE,
    IE_MAC_ERR_KEY_IDENTIFIER_MODE_MISMATCH
} IE_MacError;

const char *ie_mac_error_str(IE_MacError e);

/* A parsed MAC frame. `payload` is heap-owned (freed by ie_mac_frame_free). */
typedef struct {
    IE_FrameControl frame_control;
    int has_sequence_number;
    uint8_t sequence_number;
    int has_destination_pan_id;
    uint16_t destination_pan_id;
    int has_destination;
    IE_Address destination;
    int has_source_pan_id;
    uint16_t source_pan_id;
    int has_source;
    IE_Address source;
    int has_aux_security_header;
    IE_AuxSecurityHeader aux_security_header;
    uint8_t *payload; /* owned; may be NULL when payload_len == 0 */
    size_t payload_len;
    int has_fcs;
    uint16_t fcs;
} IE_MacFrame;

/* Body-free read model of the frame shape. */
typedef struct {
    IE_FrameType frame_type;
    IE_FrameVersion frame_version;
    IE_AddressMode destination_address_mode;
    IE_AddressMode source_address_mode;
    int security_enabled;
    int has_auxiliary_security_header;
    int ack_request;
    int frame_pending;
    int pan_id_compression;
    int sequence_number_suppressed;
    int information_elements_present;
    int has_sequence_number;
    int has_destination_pan_id;
    int has_source_pan_id;
    int has_destination;
    int has_source;
    size_t payload_len;
    int has_fcs;
} IE_MacFrameSummary;

/* Parse a MAC frame (optionally trailed by a 2-byte FCS). On IE_MAC_OK, *out
 * owns a payload buffer; free with ie_mac_frame_free. */
IE_MacError ie_mac_frame_parse(const uint8_t *bytes, size_t len, int has_fcs,
                               IE_MacFrame *out);
/* Encode a MAC frame. On IE_MAC_OK, allocates *out_bytes (caller frees). */
IE_MacError ie_mac_frame_encode(const IE_MacFrame *frame, uint8_t **out_bytes,
                                size_t *out_len);
void ie_mac_frame_summary(const IE_MacFrame *frame, IE_MacFrameSummary *out);
void ie_mac_frame_free(IE_MacFrame *frame);
int ie_mac_summary_has_payload(const IE_MacFrameSummary *s);
int ie_mac_summary_has_addressing(const IE_MacFrameSummary *s);

/* Superframe specification accessors (operate on the raw 16-bit field). */
uint8_t ie_superframe_beacon_order(uint16_t raw);
uint8_t ie_superframe_order(uint16_t raw);
uint8_t ie_superframe_final_cap_slot(uint16_t raw);
int ie_superframe_battery_life_extension(uint16_t raw);
int ie_superframe_pan_coordinator(uint16_t raw);
int ie_superframe_association_permit(uint16_t raw);

typedef struct {
    uint16_t short_address;
    uint8_t starting_slot;
    uint8_t length;
} IE_GtsDescriptor;

/* Beacon payload. GTS/pending arrays are bounded (≤ 7); `payload` is heap-owned
 * (freed by ie_beacon_payload_free). */
typedef struct {
    uint16_t superframe_raw;
    uint8_t gts_descriptor_count;
    int gts_permit;
    int gts_has_directions;
    uint8_t gts_directions;
    IE_GtsDescriptor gts_descriptors[7];
    uint16_t short_addresses[7];
    size_t short_address_count;
    uint64_t extended_addresses[7];
    size_t extended_address_count;
    uint8_t *payload; /* owned; may be NULL when payload_len == 0 */
    size_t payload_len;
} IE_BeaconPayload;

/* Errors from beacon payload / PAN descriptor parsing. IE_BEACON_OK (0) = ok. */
typedef enum {
    IE_BEACON_OK = 0,
    IE_BEACON_ERR_EXPECTED_BEACON_FRAME,
    IE_BEACON_ERR_MISSING_SOURCE_ADDRESS,
    IE_BEACON_ERR_MISSING_PAN_ID,
    IE_BEACON_ERR_TRUNCATED_FIELD
} IE_BeaconError;

const char *ie_beacon_error_str(IE_BeaconError e);

IE_BeaconError ie_beacon_payload_parse(const uint8_t *bytes, size_t len,
                                       IE_BeaconPayload *out);
void ie_beacon_payload_free(IE_BeaconPayload *bp);

/* A PAN descriptor. Owns its beacon payload (free with ie_pan_descriptor_free). */
typedef struct {
    uint16_t coordinator_pan_id;
    IE_Address coordinator_address;
    uint8_t channel;
    uint8_t channel_page;
    uint8_t link_quality;
    IE_BeaconPayload beacon;
} IE_PanDescriptor;

IE_BeaconError ie_pan_descriptor_from_beacon_frame(const IE_MacFrame *frame,
                                                   uint8_t channel,
                                                   uint8_t channel_page,
                                                   uint8_t link_quality,
                                                   IE_PanDescriptor *out);
int ie_pan_descriptor_association_permitted(const IE_PanDescriptor *pd);
void ie_pan_descriptor_free(IE_PanDescriptor *pd);

/* PAN scan summary helpers over an array of descriptors. */
size_t ie_pan_scan_count_for_channel(const IE_PanDescriptor *descriptors,
                                     size_t count, uint8_t channel);
size_t ie_pan_scan_association_candidate_count(
    const IE_PanDescriptor *descriptors, size_t count);
/* Index of the best (association-permitted, highest link quality, then
 * pan-coordinator) candidate, or -1 if none. */
long ie_pan_scan_best_candidate_index(const IE_PanDescriptor *descriptors,
                                      size_t count);

#ifdef __cplusplus
}
#endif

#endif /* IEEE802154_CORE_H */
