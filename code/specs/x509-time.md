# X.509 Profile Time Values

## Status

Specification for a zero-external-dependency, allocation-free decoder of the
RFC 5280 certificate and CRL time profile. It accepts one already framed
`der-asn1::Asn1Element`, enforces the exact UTCTime or GeneralizedTime tag and
wire shape required by RFC 5280, resolves the UTCTime century rule, and returns
validated Gregorian calendar fields.

This package is not a certificate parser, clock, validity-policy evaluator,
certificate-path validator, signature verifier, trust store, TLS
implementation, or network transport. A decoded time says only what instant
the field encodes; it does not say whether a certificate is currently valid.

## Standards Boundary

X.690 makes DER time encodings canonical: UTC, explicit seconds, and canonical
fraction handling. RFC 5280 narrows that grammar for Internet PKI certificates
and CRLs:

- UTCTime is exactly `YYMMDDHHMMSSZ` and uses universal primitive tag 23;
- a UTCTime year from `50` through `99` means 1950 through 1999;
- a UTCTime year from `00` through `49` means 2000 through 2049;
- GeneralizedTime is exactly `YYYYMMDDHHMMSSZ` and uses universal primitive
  tag 24;
- fractional seconds, local time, numeric UTC offsets, omitted seconds, and
  alternate punctuation are rejected;
- years through 2049 use UTCTime, while years from 2050 through 9999 use
  GeneralizedTime.

The package validates Gregorian month lengths, including the century rule for
leap years, and accepts hour 00 through 23 plus minute and second 00 through
59. It does not normalize invalid fields, accept a `24:00:00` spelling, or
consult a platform calendar API.

## Public Contract

```rust
pub enum X509TimeEncoding {
    UtcTime,
    GeneralizedTime,
}

pub struct X509Time {
    // private validated fields
}

impl X509Time {
    pub fn encoding(&self) -> X509TimeEncoding;
    pub fn year(&self) -> u16;
    pub fn month(&self) -> u8;
    pub fn day(&self) -> u8;
    pub fn hour(&self) -> u8;
    pub fn minute(&self) -> u8;
    pub fn second(&self) -> u8;
    pub fn cmp_fields(&self, other: &Self) -> core::cmp::Ordering;
}

pub fn decode_x509_time(
    element: der_asn1::Asn1Element<'_>,
) -> Result<X509Time, X509TimeError>;
```

`X509Time` is `Copy`, contains no borrowed wire bytes, and compares by the
validated year-month-day-hour-minute-second tuple. `cmp_fields` avoids an
implicit ambient clock or Unix-epoch conversion. Callers that evaluate a
certificate validity window must inject and audit their own trusted current
time in a later layer.

The decoder accepts only the two exact universal primitive tags. Its fixed
length check happens before digit parsing. Every digit is parsed directly from
the borrowed contents with checked decimal arithmetic; no text conversion,
allocation, locale, timezone database, or host calendar API is used.

## Error Contract

Errors contain one stable category and a byte offset local to the complete DER
element. They never retain or format wire bytes. Required categories are:

- unexpected class, constructed bit, or tag number;
- wrong RFC 5280 contents length;
- missing terminal `Z`;
- a non-ASCII digit;
- GeneralizedTime using a year before 2050;
- month, day, hour, minute, or second outside the accepted calendar range.

The first invalid field wins. An error cannot allocate from input, panic,
mutate the source `der-asn1` decoder, or disclose certificate bytes.

## Adversarial Matrix

Tests cover the 1949/1950 and 2049/2050 tag boundaries; 1999/2000 UTCTime
century resolution; leap days for 1996, 2000, 2100, and 2400; every fixed field
boundary; invalid month-specific days; `24:00:00`; second 60; wrong primitive
and constructed tags; truncated and extended values; omitted seconds; local
time and numeric offsets; lowercase `z`; generalized fractional seconds;
embedded non-digits; and ordering across both encodings.

## Non-Goals

This slice adds no external dependency and no ambient authority. It does not
decode generic BER time spellings, preserve fractional precision, encode DER,
parse PEM, parse a certificate schema, evaluate `notBefore`/`notAfter`, read a
clock, construct or validate certificate paths, check revocation, verify
signatures, select trust roots, match a server identity, perform a TLS
handshake, open sockets, or provide OAuth transport.
