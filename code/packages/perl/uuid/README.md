# CodingAdventures::UUID (Perl)

Pure Perl UUID v1, v3, v4, v5, and v7 generation and parsing, following
RFC 4122 / ITU-T X.667.

## What Is a UUID?

A UUID (Universally Unique Identifier) is a 128-bit value used to uniquely
identify information without central coordination. Canonical form:

```
xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx
```

Where M is the version (1–7) and N encodes the variant (`8`, `9`, `a`, `b` = RFC 4122).

## Versions Implemented

| Version | Algorithm                        | Sortable | Deterministic |
|---------|----------------------------------|----------|---------------|
| v1      | Time + random node (48-bit)      | No       | No            |
| v3      | MD5(namespace + name)            | No       | Yes           |
| v4      | 122 random bits                  | No       | No            |
| v5      | SHA-1(namespace + name)          | No       | Yes           |
| v7      | 48-bit Unix ms + random bits     | Yes      | No            |

## Usage

```perl
use CodingAdventures::UUID qw(
    generate_v4 generate_v5 generate_v3
    generate_v1 generate_v7
    parse validate nil_uuid
);

# v4: random UUID (most common)
my $u = generate_v4();
# e.g. "3d813cbb-47fb-32ba-91df-831e1593ac29"

# v5: deterministic from a name (preferred over v3)
my $u = generate_v5($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
# Always: "2ed6657d-e927-568b-95e3-af9f787f5a91"

# v3: deterministic from a name (MD5 variant)
my $u = generate_v3($CodingAdventures::UUID::NAMESPACE_DNS, "www.example.com");
# Always: "5df41881-3aed-3515-88a7-2f4a814cf09e"

# validate
validate("550e8400-e29b-41d4-a716-446655440000");  # true
validate("not-a-uuid");                             # false

# parse
my $info = parse(generate_v4());
print $info->{version};  # 4
print $info->{variant};  # "rfc4122"

# nil UUID
nil_uuid();  # "00000000-0000-0000-0000-000000000000"
```

## Namespace Constants

```perl
$CodingAdventures::UUID::NAMESPACE_DNS   # "6ba7b810-9dad-11d1-80b4-00c04fd430c8"
$CodingAdventures::UUID::NAMESPACE_URL   # "6ba7b811-9dad-11d1-80b4-00c04fd430c8"
```

## RFC 4122 Test Vectors

Verified by the test suite (Appendix B):

```
v3(NAMESPACE_DNS, "www.example.com") = "5df41881-3aed-3515-88a7-2f4a814cf09e"
v5(NAMESPACE_DNS, "www.example.com") = "2ed6657d-e927-568b-95e1-2665a8aea6a2"
```

## Installation

```bash
# Install dependencies first
cd ../md5  && cpanm --notest .
cd ../sha1 && cpanm --notest .
# Then install uuid
cd ../uuid && cpanm .
```

## Running Tests

```bash
prove -l -v t/
```

## Dependencies

- `CodingAdventures::Md5` — used by `generate_v3`
- `CodingAdventures::Sha1` — used by `generate_v5`
