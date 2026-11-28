/*
 * HTTP/0.9 in-memory codec for the early Web profile (HWEB00).
 *
 * Every operation works on explicit byte lengths. This keeps the codec
 * independent of NUL termination, sockets, stdio, and platform text modes.
 * Successful nonempty results own one malloc allocation, released with
 * hweb_bytes_free; on any failure the caller-visible output stays empty.
 */
#include "historical_web.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* One helper keeps optional error reporting consistent and prevents a stale
 * diagnostic from surviving a successful call. */
static void set_error(HwebError *error, HwebStatus status, size_t byte_offset) {
    if (error != NULL) {
        error->status = status;
        error->byte_offset = byte_offset;
    }
}

static HwebStatus fail(HwebError *error, HwebStatus status, size_t byte_offset) {
    set_error(error, status, byte_offset);
    return status;
}

static int printable_address_byte(uint8_t byte) {
    return byte >= 0x21u && byte <= 0x7eu;
}

HwebStatus hweb_http09_encode_request(const uint8_t *address,
                                      size_t address_length,
                                      HwebBytes *out, HwebError *error) {
    static const uint8_t prefix[] = {0x47u, 0x45u, 0x54u, 0x20u};
    static const uint8_t suffix[] = {0x0du, 0x0au};
    size_t wire_length;
    uint8_t *wire;
    size_t i;

    if (out == NULL) {
        set_error(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
        return HWEB_ERR_ARGUMENT;
    }
    set_error(error, HWEB_OK, HWEB_NO_INDEX);
    if (out->data != NULL || out->length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }

    if (address == NULL && address_length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }
    if (address_length == 0) {
        return fail(error, HWEB_ERR_PROTOCOL, HWEB_NO_INDEX);
    }

    /* The exact wire length is address_length + 4 prefix + 2 CRLF bytes.
     * Guard before the data loop so malformed size values cannot wrap. */
    if (address_length > SIZE_MAX - 6u) {
        return fail(error, HWEB_ERR_OVERFLOW, HWEB_NO_INDEX);
    }
    wire_length = address_length + 6u;

    for (i = 0; i < address_length; i++) {
        if (!printable_address_byte(address[i])) {
            return fail(error, HWEB_ERR_PROTOCOL, i);
        }
    }

    wire = (uint8_t *)malloc(wire_length);
    if (wire == NULL) {
        return fail(error, HWEB_ERR_NO_MEMORY, HWEB_NO_INDEX);
    }
    memcpy(wire, prefix, sizeof(prefix));
    memcpy(wire + sizeof(prefix), address, address_length);
    memcpy(wire + sizeof(prefix) + address_length, suffix, sizeof(suffix));
    out->data = wire;
    out->length = wire_length;
    return HWEB_OK;
}

HwebStatus hweb_http09_parse_request(const uint8_t *request,
                                     size_t request_length,
                                     HwebBytes *out, HwebError *error) {
    static const uint8_t prefix[] = {0x47u, 0x45u, 0x54u, 0x20u};
    size_t line_end;
    size_t address_length;
    size_t i;
    uint8_t *address;

    if (out == NULL) {
        set_error(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
        return HWEB_ERR_ARGUMENT;
    }
    set_error(error, HWEB_OK, HWEB_NO_INDEX);
    if (out->data != NULL || out->length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }

    if (request == NULL && request_length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }
    if (request_length < sizeof(prefix)) {
        return fail(error, HWEB_ERR_PROTOCOL, request_length);
    }
    for (i = 0; i < sizeof(prefix); i++) {
        if (request[i] != prefix[i]) {
            return fail(error, HWEB_ERR_PROTOCOL, i);
        }
    }
    if (request[request_length - 1] != 0x0au) {
        return fail(error, HWEB_ERR_PROTOCOL, request_length);
    }

    /* Accept LF or CRLF, removing only the terminator at the end. Any second
     * line or stray CR remains in the address span and fails printable ASCII. */
    line_end = request_length - 1;
    if (line_end > sizeof(prefix) && request[line_end - 1] == 0x0du) {
        line_end--;
    }
    address_length = line_end - sizeof(prefix);
    if (address_length == 0) {
        return fail(error, HWEB_ERR_PROTOCOL, HWEB_NO_INDEX);
    }
    for (i = 0; i < address_length; i++) {
        if (!printable_address_byte(request[sizeof(prefix) + i])) {
            return fail(error, HWEB_ERR_PROTOCOL, sizeof(prefix) + i);
        }
    }

    address = (uint8_t *)malloc(address_length);
    if (address == NULL) {
        return fail(error, HWEB_ERR_NO_MEMORY, HWEB_NO_INDEX);
    }
    memcpy(address, request + sizeof(prefix), address_length);
    out->data = address;
    out->length = address_length;
    return HWEB_OK;
}

HwebStatus hweb_http09_decode_response(const uint8_t *response,
                                       size_t response_length,
                                       HwebBytes *out, HwebError *error) {
    size_t i;
    uint8_t *body;

    if (out == NULL) {
        set_error(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
        return HWEB_ERR_ARGUMENT;
    }
    set_error(error, HWEB_OK, HWEB_NO_INDEX);
    if (out->data != NULL || out->length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }

    if (response == NULL && response_length != 0) {
        return fail(error, HWEB_ERR_ARGUMENT, HWEB_NO_INDEX);
    }
    for (i = 0; i < response_length; i++) {
        if (response[i] > 0x7fu) {
            return fail(error, HWEB_ERR_PROTOCOL, i);
        }
    }
    if (response_length == 0) {
        return HWEB_OK;
    }

    body = (uint8_t *)malloc(response_length);
    if (body == NULL) {
        return fail(error, HWEB_ERR_NO_MEMORY, HWEB_NO_INDEX);
    }
    memcpy(body, response, response_length);
    out->data = body;
    out->length = response_length;
    return HWEB_OK;
}

void hweb_bytes_free(HwebBytes *bytes) {
    if (bytes == NULL) {
        return;
    }
    free(bytes->data);
    bytes->data = NULL;
    bytes->length = 0;
}
