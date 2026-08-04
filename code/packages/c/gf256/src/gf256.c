/*
 * gf256.c — implementation of GF(2^8) arithmetic (see gf256.h). A faithful port
 * of the Rust `gf256` crate: log/antilog tables for the default 0x11D field and
 * table-free Russian-peasant multiplication for the parameterisable field.
 */
#include "gf256.h"

/* ── log / antilog tables for the default field (polynomial 0x11D) ────────── */
static uint16_t g_log[256];
static uint8_t g_alog[256];
static int g_ready = 0;

/* Build the tables: ALOG[i] = g^i (g = 2), LOG[ALOG[i]] = i. Multiplying by g
 * is a left shift; a carry out of bit 7 is reduced by XOR with the polynomial. */
static void ensure_tables(void) {
    uint16_t val = 1;
    int i;
    if (g_ready) {
        return;
    }
    for (i = 0; i < 255; i++) {
        g_alog[i] = (uint8_t)val;
        g_log[val] = (uint16_t)i;
        val = (uint16_t)(val << 1);
        if (val >= 256) {
            val ^= GF256_PRIMITIVE_POLYNOMIAL;
        }
    }
    g_alog[255] = 1; /* g^255 = g^0 = 1 (group order 255) */
    g_log[0] = 0;    /* undefined; never read for a valid computation */
    g_ready = 1;
}

uint8_t gf256_add(uint8_t a, uint8_t b) { return (uint8_t)(a ^ b); }
uint8_t gf256_subtract(uint8_t a, uint8_t b) { return (uint8_t)(a ^ b); }

uint8_t gf256_multiply(uint8_t a, uint8_t b) {
    unsigned exp;
    if (a == 0 || b == 0) {
        return 0;
    }
    ensure_tables();
    exp = ((unsigned)g_log[a] + (unsigned)g_log[b]) % 255u;
    return g_alog[exp];
}

uint8_t gf256_divide(uint8_t a, uint8_t b) {
    int exp;
    if (b == 0 || a == 0) {
        return 0;
    }
    ensure_tables();
    exp = ((int)g_log[a] - (int)g_log[b] + 255) % 255;
    return g_alog[exp];
}

uint8_t gf256_power(uint8_t base, uint32_t exp) {
    unsigned e;
    if (base == 0) {
        return (uint8_t)(exp == 0 ? 1 : 0);
    }
    if (exp == 0) {
        return 1;
    }
    ensure_tables();
    e = (unsigned)(((uint64_t)g_log[base] * (uint64_t)exp) % 255u);
    return g_alog[e];
}

uint8_t gf256_inverse(uint8_t a) {
    if (a == 0) {
        return 0;
    }
    ensure_tables();
    return g_alog[255 - g_log[a]];
}

/* ── parameterisable field (Russian-peasant, any polynomial) ──────────────── */
gf256_field gf256_field_new(uint16_t primitive_poly) {
    gf256_field f;
    f.primitive_polynomial = primitive_poly;
    f.reduce = (uint8_t)(primitive_poly & 0xff);
    return f;
}

static uint8_t field_mul(const gf256_field *f, uint8_t a, uint8_t b) {
    uint8_t result = 0;
    uint8_t aa = a;
    uint8_t bb = b;
    int i;
    for (i = 0; i < 8; i++) {
        if (bb & 1) {
            result = (uint8_t)(result ^ aa);
        }
        {
            uint8_t hi = (uint8_t)(aa & 0x80);
            aa = (uint8_t)(aa << 1);
            if (hi) {
                aa = (uint8_t)(aa ^ f->reduce);
            }
        }
        bb = (uint8_t)(bb >> 1);
    }
    return result;
}

static uint8_t field_pow(const gf256_field *f, uint8_t base, uint32_t exp) {
    uint8_t result = 1;
    uint8_t b = base;
    uint32_t e = exp;
    if (base == 0) {
        return (uint8_t)(exp == 0 ? 1 : 0);
    }
    if (exp == 0) {
        return 1;
    }
    while (e > 0) {
        if (e & 1) {
            result = field_mul(f, result, b);
        }
        b = field_mul(f, b, b);
        e >>= 1;
    }
    return result;
}

uint8_t gf256_field_add(const gf256_field *f, uint8_t a, uint8_t b) {
    (void)f;
    return (uint8_t)(a ^ b);
}
uint8_t gf256_field_subtract(const gf256_field *f, uint8_t a, uint8_t b) {
    (void)f;
    return (uint8_t)(a ^ b);
}
uint8_t gf256_field_multiply(const gf256_field *f, uint8_t a, uint8_t b) {
    return field_mul(f, a, b);
}
uint8_t gf256_field_divide(const gf256_field *f, uint8_t a, uint8_t b) {
    if (b == 0) {
        return 0;
    }
    return field_mul(f, a, field_pow(f, b, 254));
}
uint8_t gf256_field_power(const gf256_field *f, uint8_t base, uint32_t exp) {
    return field_pow(f, base, exp);
}
uint8_t gf256_field_inverse(const gf256_field *f, uint8_t a) {
    if (a == 0) {
        return 0;
    }
    return field_pow(f, a, 254);
}
