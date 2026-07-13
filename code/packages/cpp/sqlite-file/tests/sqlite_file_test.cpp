// sqlite_file_test.cpp — unit tests for the C++ SQLite-file reader.
//
// Mirrors the Rust crate's suite across all modules: varint golden vectors +
// round-trip sweep + truncation, record decode cases, header parsing, the
// pager, and b-tree table/index walks including overflow reassembly, cycle
// detection, and the anti-amplification guard.  Database fixtures are built as
// byte arrays exactly as the crate's tests build them.
#include "sqlite_file.hpp"
#include "iso_test.h"

#include <cstring>
#include <vector>

namespace sf = ca::sqlite_file;

namespace {

void put_be16(std::vector<std::uint8_t>& b, std::size_t off, std::uint16_t v) {
    b[off] = static_cast<std::uint8_t>(v >> 8);
    b[off + 1] = static_cast<std::uint8_t>(v & 0xff);
}
void put_be32(std::vector<std::uint8_t>& b, std::size_t off, std::uint32_t v) {
    b[off] = static_cast<std::uint8_t>(v >> 24);
    b[off + 1] = static_cast<std::uint8_t>(v >> 16);
    b[off + 2] = static_cast<std::uint8_t>(v >> 8);
    b[off + 3] = static_cast<std::uint8_t>(v & 0xff);
}
void put_magic(std::vector<std::uint8_t>& b) {
    std::memcpy(b.data(), sf::detail::MAGIC, 16);
}

template <typename F>
bool throws_code(sf::Error expected, F&& fn) {
    try {
        fn();
    } catch (const sf::SqliteError& e) {
        return e.code() == expected;
    } catch (...) {
        return false;
    }
    return false;
}

// ── varint ────────────────────────────────────────────────────────

void test_varint_golden() {
    struct G { std::int64_t v; std::vector<std::uint8_t> bytes; };
    const G golden[] = {
        {0, {0x00}},        {1, {0x01}},       {127, {0x7f}},
        {128, {0x81, 0x00}}, {129, {0x81, 0x01}}, {255, {0x81, 0x7f}},
        {256, {0x82, 0x00}}, {300, {0x82, 0x2c}}, {16383, {0xff, 0x7f}},
        {16384, {0x81, 0x80, 0x00}}, {2097151, {0xff, 0xff, 0x7f}},
    };
    for (const G& g : golden) {
        std::vector<std::uint8_t> out;
        std::size_t n = sf::varint::write(g.v, out);
        ISO_CHECK_EQ_UINT(n, g.bytes.size());
        ISO_CHECK(out == g.bytes);
        auto r = sf::varint::read(g.bytes.data(), g.bytes.size());
        ISO_CHECK(r.has_value());
        ISO_CHECK(r->first == g.v);
        ISO_CHECK_EQ_UINT(r->second, g.bytes.size());
    }
}

void test_varint_max_u64() {
    std::vector<std::uint8_t> out;
    std::size_t n = sf::varint::write(-1, out); // u64::MAX bit pattern
    ISO_CHECK_EQ_UINT(n, 9u);
    std::vector<std::uint8_t> expected(9, 0xff);
    ISO_CHECK(out == expected);
    auto r = sf::varint::read(out.data(), out.size());
    ISO_CHECK(r.has_value() && r->first == -1 && r->second == 9);
}

void test_varint_sweep_and_truncation() {
    std::uint64_t state = 0x123456789abcdef1ULL;
    for (int i = 0; i < 50000; ++i) {
        state = state * 6364136223846793005ULL + 1442695040888963407ULL;
        std::int64_t value = static_cast<std::int64_t>(state);
        std::vector<std::uint8_t> out;
        std::size_t n = sf::varint::write(value, out);
        auto r = sf::varint::read(out.data(), out.size());
        ISO_CHECK(r.has_value() && r->first == value && r->second == n);
        ISO_CHECK(n >= 1 && n <= 9);
    }
    std::uint8_t cont = 0x81;
    ISO_CHECK(!sf::varint::read(&cont, 1).has_value());
    ISO_CHECK(!sf::varint::read(nullptr, 0).has_value());
    std::uint8_t eight[8];
    std::memset(eight, 0x80, 8);
    ISO_CHECK(!sf::varint::read(eight, 8).has_value());
}

// ── record ────────────────────────────────────────────────────────

void test_record_decode() {
    // [NULL, 42, "hi"]
    std::vector<std::uint8_t> r1 = {0x04, 0x00, 0x01, 0x11, 0x2a, 0x68, 0x69};
    auto v1 = sf::record::decode(r1);
    ISO_CHECK(v1.has_value() && v1->size() == 3);
    ISO_CHECK((*v1)[0].index() == sf::record::VNull);
    ISO_CHECK((*v1)[1] == sf::Value{static_cast<std::int64_t>(42)});
    ISO_CHECK((*v1)[2] == sf::Value{std::string("hi")});

    // [0, 1, 1.5]
    std::vector<std::uint8_t> r2 = {0x04, 0x08, 0x09, 0x07, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0};
    auto v2 = sf::record::decode(r2);
    ISO_CHECK(v2.has_value());
    ISO_CHECK((*v2)[0] == sf::Value{static_cast<std::int64_t>(0)});
    ISO_CHECK((*v2)[1] == sf::Value{static_cast<std::int64_t>(1)});
    ISO_CHECK((*v2)[2] == sf::Value{1.5});

    // negatives sign-extend
    ISO_CHECK((*sf::record::decode(std::vector<std::uint8_t>{0x02, 0x02, 0xff, 0xfe}))[0] ==
              sf::Value{static_cast<std::int64_t>(-2)});
    ISO_CHECK((*sf::record::decode(std::vector<std::uint8_t>{0x02, 0x01, 0xff}))[0] ==
              sf::Value{static_cast<std::int64_t>(-1)});

    // blob
    ISO_CHECK(((*sf::record::decode(std::vector<std::uint8_t>{0x02, 0x10, 0xde, 0xad}))[0] ==
               sf::Value{std::vector<std::uint8_t>{0xde, 0xad}}));

    // wide widths
    ISO_CHECK((*sf::record::decode(std::vector<std::uint8_t>{0x02, 0x03, 0x01, 0x00, 0x00}))[0] ==
              sf::Value{static_cast<std::int64_t>(65536)});
    ISO_CHECK((*sf::record::decode(std::vector<std::uint8_t>{0x02, 0x06, 0, 0, 0, 1, 0, 0, 0, 0}))[0] ==
              sf::Value{static_cast<std::int64_t>(1LL << 32)});

    // corrupt
    ISO_CHECK(!sf::record::decode(std::vector<std::uint8_t>{0x04}).has_value());
    ISO_CHECK(!sf::record::decode(std::vector<std::uint8_t>{0x02, 0x06, 0x00}).has_value());
    ISO_CHECK(!sf::record::decode(std::vector<std::uint8_t>{0x02, 0x0a}).has_value());
}

// ── header ────────────────────────────────────────────────────────

std::vector<std::uint8_t> make_header(std::uint16_t page_size_field, std::uint32_t encoding) {
    std::vector<std::uint8_t> buf(100, 0);
    put_magic(buf);
    put_be16(buf, 16, page_size_field);
    put_be32(buf, 56, encoding);
    return buf;
}

void test_header() {
    auto buf = make_header(4096, 1);
    put_be32(buf, 28, 7);
    buf[20] = 0;
    sf::Header h = sf::parse_header(buf);
    ISO_CHECK_EQ_UINT(h.page_size, 4096u);
    ISO_CHECK_EQ_UINT(h.page_count, 7u);
    ISO_CHECK_EQ_UINT(h.reserved_space, 0u);
    ISO_CHECK(h.text_encoding == sf::TextEncoding::Utf8);
    ISO_CHECK_EQ_UINT(h.usable_size(), 4096u);

    ISO_CHECK_EQ_UINT(sf::parse_header(make_header(1, 1)).page_size, 65536u);

    auto rbuf = make_header(4096, 1);
    rbuf[20] = 32;
    ISO_CHECK_EQ_UINT(sf::parse_header(rbuf).usable_size(), 4096u - 32u);

    auto bad = make_header(4096, 1);
    bad[0] = 'X';
    ISO_CHECK(throws_code(sf::Error::BadMagic, [&] { sf::parse_header(bad); }));
    ISO_CHECK(throws_code(sf::Error::BadPageSize, [&] { sf::parse_header(make_header(4097, 1)); }));
    ISO_CHECK(throws_code(sf::Error::BadPageSize, [&] { sf::parse_header(make_header(256, 1)); }));
    ISO_CHECK(throws_code(sf::Error::Truncated, [&] {
        std::vector<std::uint8_t> tiny(50, 0);
        sf::parse_header(tiny);
    }));
    ISO_CHECK(sf::parse_header(make_header(4096, 2)).text_encoding == sf::TextEncoding::Utf16Le);
    ISO_CHECK(sf::parse_header(make_header(4096, 3)).text_encoding == sf::TextEncoding::Utf16Be);
    ISO_CHECK(throws_code(sf::Error::Unsupported, [&] { sf::parse_header(make_header(4096, 9)); }));
}

// ── pager ─────────────────────────────────────────────────────────

std::vector<std::uint8_t> three_page_db(std::size_t ps) {
    std::vector<std::uint8_t> data(ps * 3, 0);
    put_magic(data);
    put_be16(data, 16, static_cast<std::uint16_t>(ps));
    put_be32(data, 56, 1);
    put_be32(data, 28, 3);
    data[100] = 0xA1;
    data[ps] = 0xB2;
    data[ps * 2] = 0xC3;
    return data;
}

void test_pager() {
    auto db = three_page_db(512);
    auto ho = sf::Pager::open(db);
    ISO_CHECK_EQ_UINT(ho.first.page_size, 512u);
    ISO_CHECK_EQ_UINT(ho.first.page_count, 3u);
    ISO_CHECK_EQ_UINT(ho.second.page_size(), 512u);
    ISO_CHECK_EQ_UINT(ho.second.page_count(), 3u);

    auto p1 = ho.second.page(1);
    ISO_CHECK_EQ_UINT(p1.second, 512u);
    ISO_CHECK(std::memcmp(p1.first, sf::detail::MAGIC, 16) == 0);
    ISO_CHECK_EQ_UINT(p1.first[100], 0xA1u);
    ISO_CHECK_EQ_UINT(ho.second.page(2).first[0], 0xB2u);
    ISO_CHECK_EQ_UINT(ho.second.page(3).first[0], 0xC3u);

    ISO_CHECK(throws_code(sf::Error::BadPageNumber, [&] { ho.second.page(0); }));
    ISO_CHECK(throws_code(sf::Error::BadPageNumber, [&] { ho.second.page(4); }));
    ISO_CHECK(throws_code(sf::Error::BadPageNumber, [&] { ho.second.page(0xFFFFFFFFu); }));
}

// ── btree ─────────────────────────────────────────────────────────

std::vector<std::uint8_t> one_leaf_page_db(
    std::size_t ps, const std::vector<std::pair<std::int64_t, std::vector<std::uint8_t>>>& rows) {
    std::vector<std::uint8_t> page(ps, 0);
    put_magic(page);
    put_be16(page, 16, static_cast<std::uint16_t>(ps));
    put_be32(page, 56, 1);
    put_be32(page, 28, 1);
    std::size_t h = 100;
    page[h] = sf::detail::LEAF_TABLE;
    put_be16(page, h + 3, static_cast<std::uint16_t>(rows.size()));
    std::size_t content_top = ps;
    std::size_t ptr_array = h + 8;
    for (std::size_t i = 0; i < rows.size(); ++i) {
        std::vector<std::uint8_t> cell;
        sf::varint::write(static_cast<std::int64_t>(rows[i].second.size()), cell);
        sf::varint::write(rows[i].first, cell);
        cell.insert(cell.end(), rows[i].second.begin(), rows[i].second.end());
        content_top -= cell.size();
        std::memcpy(page.data() + content_top, cell.data(), cell.size());
        put_be16(page, ptr_array + i * 2, static_cast<std::uint16_t>(content_top));
    }
    put_be16(page, h + 5, static_cast<std::uint16_t>(content_top));
    return page;
}

void test_btree_leaf() {
    auto db = one_leaf_page_db(512, {{2, {0xAA, 0xBB}}, {1, {0x01}}, {3, {0xCC, 0xDD, 0xEE}}});
    auto ho = sf::Pager::open(db);
    auto rows = sf::walk_table(ho.second, ho.first, 1);
    ISO_CHECK(rows.size() == 3);
    ISO_CHECK((rows[0].first == 1 && rows[0].second == std::vector<std::uint8_t>{0x01}));
    ISO_CHECK((rows[1].first == 2 && rows[1].second == std::vector<std::uint8_t>{0xAA, 0xBB}));
    ISO_CHECK((rows[2].first == 3 && rows[2].second == std::vector<std::uint8_t>{0xCC, 0xDD, 0xEE}));

    auto empty = one_leaf_page_db(512, {});
    auto he = sf::Pager::open(empty);
    ISO_CHECK(sf::walk_table(he.second, he.first, 1).empty());
}

std::vector<std::uint8_t> interior_over_two_leaves_db() {
    std::size_t ps = 512;
    std::vector<std::uint8_t> data(ps * 3, 0);
    put_magic(data);
    put_be16(data, 16, static_cast<std::uint16_t>(ps));
    put_be32(data, 56, 1);
    put_be32(data, 28, 3);
    std::size_t h = 100;
    data[h] = sf::detail::INTERIOR_TABLE;
    put_be16(data, h + 3, 1);
    put_be32(data, h + 8, 3); // right child = page 3
    std::vector<std::uint8_t> cell;
    cell.push_back(0); cell.push_back(0); cell.push_back(0); cell.push_back(2); // left child = 2 (be32)
    sf::varint::write(2, cell);
    std::size_t cell_off = ps - cell.size();
    std::memcpy(data.data() + cell_off, cell.data(), cell.size());
    put_be16(data, h + 12, static_cast<std::uint16_t>(cell_off));

    struct L { std::uint32_t page_no; std::vector<std::pair<std::int64_t, std::vector<std::uint8_t>>> rows; };
    L leaves[] = {{2, {{1, {0x11}}}}, {3, {{2, {0x22}}, {3, {0x33}}}}};
    for (const L& l : leaves) {
        std::size_t base = (l.page_no - 1) * ps;
        data[base] = sf::detail::LEAF_TABLE;
        put_be16(data, base + 3, static_cast<std::uint16_t>(l.rows.size()));
        std::size_t top = base + ps;
        for (std::size_t i = 0; i < l.rows.size(); ++i) {
            std::vector<std::uint8_t> c;
            sf::varint::write(static_cast<std::int64_t>(l.rows[i].second.size()), c);
            sf::varint::write(l.rows[i].first, c);
            c.insert(c.end(), l.rows[i].second.begin(), l.rows[i].second.end());
            top -= c.size();
            std::memcpy(data.data() + top, c.data(), c.size());
            put_be16(data, base + 8 + i * 2, static_cast<std::uint16_t>(top - base));
        }
    }
    return data;
}

void test_btree_interior_and_errors() {
    auto db = interior_over_two_leaves_db();
    auto ho = sf::Pager::open(db);
    auto rows = sf::walk_table(ho.second, ho.first, 1);
    ISO_CHECK(rows.size() == 3);
    ISO_CHECK(rows[0].second == std::vector<std::uint8_t>{0x11});
    ISO_CHECK(rows[1].second == std::vector<std::uint8_t>{0x22});
    ISO_CHECK(rows[2].second == std::vector<std::uint8_t>{0x33});

    auto bad = one_leaf_page_db(512, {{1, {0x00}}});
    bad[100] = 0x0A; // index leaf, not a table page
    auto hb = sf::Pager::open(bad);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_table(hb.second, hb.first, 1); }));

    // interior page whose right child points at itself → cycle
    std::size_t ps = 512;
    std::vector<std::uint8_t> data(ps, 0);
    put_magic(data);
    put_be16(data, 16, static_cast<std::uint16_t>(ps));
    put_be32(data, 56, 1);
    put_be32(data, 28, 1);
    data[100] = sf::detail::INTERIOR_TABLE;
    put_be16(data, 103, 0);
    put_be32(data, 108, 1); // right child = self
    auto hc = sf::Pager::open(data);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_table(hc.second, hc.first, 1); }));
}

std::size_t table_inline_len(std::size_t usable, std::size_t payload_len) {
    std::size_t max_local = usable - 35;
    std::size_t min_local = (usable - 12) * 32 / 255 - 23;
    std::size_t span = usable - 4;
    std::size_t k = min_local + (payload_len - min_local) % span;
    return (k <= max_local) ? k : min_local;
}

std::vector<std::uint8_t> one_overflow_row_db(std::size_t ps, std::int64_t rowid,
                                              const std::vector<std::uint8_t>& payload) {
    std::size_t usable = ps;
    std::size_t inline_len = table_inline_len(usable, payload.size());
    std::vector<std::uint8_t> head(payload.begin(), payload.begin() + static_cast<std::ptrdiff_t>(inline_len));
    std::vector<std::uint8_t> tail(payload.begin() + static_cast<std::ptrdiff_t>(inline_len), payload.end());
    std::size_t content = usable - 4;
    std::size_t n_overflow = (tail.size() + content - 1) / content;
    std::size_t total_pages = 2 + n_overflow;
    std::uint32_t first_overflow = 3;

    std::vector<std::uint8_t> data(ps * total_pages, 0);
    put_magic(data);
    put_be16(data, 16, static_cast<std::uint16_t>(ps));
    put_be32(data, 56, 1);
    put_be32(data, 28, static_cast<std::uint32_t>(total_pages));

    std::size_t base = ps;
    data[base] = sf::detail::LEAF_TABLE;
    put_be16(data, base + 3, 1);
    std::vector<std::uint8_t> cell;
    sf::varint::write(static_cast<std::int64_t>(payload.size()), cell);
    sf::varint::write(rowid, cell);
    cell.insert(cell.end(), head.begin(), head.end());
    cell.push_back(static_cast<std::uint8_t>(first_overflow >> 24));
    cell.push_back(static_cast<std::uint8_t>(first_overflow >> 16));
    cell.push_back(static_cast<std::uint8_t>(first_overflow >> 8));
    cell.push_back(static_cast<std::uint8_t>(first_overflow));
    std::size_t cell_rel = ps - cell.size();
    std::memcpy(data.data() + base + cell_rel, cell.data(), cell.size());
    put_be16(data, base + 8, static_cast<std::uint16_t>(cell_rel));
    put_be16(data, base + 5, static_cast<std::uint16_t>(cell_rel));

    for (std::size_t i = 0; i < n_overflow; ++i) {
        std::size_t page_no = first_overflow + i;
        std::size_t ob = (page_no - 1) * ps;
        std::uint32_t next = (i + 1 < n_overflow) ? static_cast<std::uint32_t>(page_no + 1) : 0;
        put_be32(data, ob, next);
        std::size_t off = i * content;
        std::size_t chunk = tail.size() - off < content ? tail.size() - off : content;
        std::memcpy(data.data() + ob + 4, tail.data() + off, chunk);
    }
    return data;
}

void test_btree_overflow() {
    std::vector<std::uint8_t> payload(1500);
    for (std::size_t i = 0; i < 1500; ++i) payload[i] = static_cast<std::uint8_t>(i % 251);
    auto db = one_overflow_row_db(512, 7, payload);
    auto ho = sf::Pager::open(db);
    auto rows = sf::walk_table(ho.second, ho.first, 2);
    ISO_CHECK(rows.size() == 1 && rows[0].first == 7 && rows[0].second == payload);

    std::vector<std::uint8_t> big(5000);
    for (std::size_t i = 0; i < 5000; ++i) big[i] = static_cast<std::uint8_t>(i * 7 % 256);
    auto db2 = one_overflow_row_db(512, 1, big);
    auto ho2 = sf::Pager::open(db2);
    ISO_CHECK(sf::walk_table(ho2.second, ho2.first, 2)[0].second == big);

    // cycle
    auto dbc = one_overflow_row_db(512, 1, payload);
    put_be32(dbc, 1024, 3); // page 3 points at itself
    auto hc = sf::Pager::open(dbc);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_table(hc.second, hc.first, 2); }));

    // ends too soon
    auto dbe = one_overflow_row_db(512, 1, payload);
    put_be32(dbe, 1024, 0);
    auto he = sf::Pager::open(dbe);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_table(he.second, he.first, 2); }));
}

void test_amplification_guard() {
    std::size_t ps = 512;
    std::vector<std::uint8_t> page(ps, 0);
    put_magic(page);
    put_be16(page, 16, static_cast<std::uint16_t>(ps));
    put_be32(page, 56, 1);
    put_be32(page, 28, 1);
    std::size_t h = 100;
    page[h] = sf::detail::LEAF_TABLE;
    put_be16(page, h + 3, 20); // 20 cells claimed
    std::vector<std::uint8_t> record(400, 0x5A);
    std::vector<std::uint8_t> cell;
    sf::varint::write(400, cell);
    sf::varint::write(1, cell);
    cell.insert(cell.end(), record.begin(), record.end());
    std::size_t cell_off = ps - cell.size();
    std::memcpy(page.data() + cell_off, cell.data(), cell.size());
    for (std::size_t i = 0; i < 20; ++i) put_be16(page, h + 8 + i * 2, static_cast<std::uint16_t>(cell_off));
    auto ho = sf::Pager::open(page);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_table(ho.second, ho.first, 1); }));
}

std::vector<std::uint8_t> one_leaf_index_db(std::size_t ps,
                                            const std::vector<std::vector<std::uint8_t>>& records) {
    std::vector<std::uint8_t> page(ps, 0);
    put_magic(page);
    put_be16(page, 16, static_cast<std::uint16_t>(ps));
    put_be32(page, 56, 1);
    put_be32(page, 28, 1);
    std::size_t h = 100;
    page[h] = sf::detail::LEAF_INDEX;
    put_be16(page, h + 3, static_cast<std::uint16_t>(records.size()));
    std::size_t top = ps;
    std::size_t ptr_array = h + 8;
    for (std::size_t i = 0; i < records.size(); ++i) {
        std::vector<std::uint8_t> cell;
        sf::varint::write(static_cast<std::int64_t>(records[i].size()), cell);
        cell.insert(cell.end(), records[i].begin(), records[i].end());
        top -= cell.size();
        std::memcpy(page.data() + top, cell.data(), cell.size());
        put_be16(page, ptr_array + i * 2, static_cast<std::uint16_t>(top));
    }
    put_be16(page, h + 5, static_cast<std::uint16_t>(top));
    return page;
}

void test_btree_index() {
    auto db = one_leaf_index_db(512, {{0x01, 0x02}, {0xAA}, {0xBB, 0xCC, 0xDD}});
    auto ho = sf::Pager::open(db);
    auto got = sf::walk_index(ho.second, ho.first, 1);
    std::sort(got.begin(), got.end());
    std::vector<std::vector<std::uint8_t>> want = {{0x01, 0x02}, {0xAA}, {0xBB, 0xCC, 0xDD}};
    std::sort(want.begin(), want.end());
    ISO_CHECK(got == want);

    auto empty = one_leaf_index_db(512, {});
    auto he = sf::Pager::open(empty);
    ISO_CHECK(sf::walk_index(he.second, he.first, 1).empty());

    // interior index emits divider + children
    std::size_t ps = 512;
    std::vector<std::uint8_t> data(ps * 3, 0);
    put_magic(data);
    put_be16(data, 16, static_cast<std::uint16_t>(ps));
    put_be32(data, 56, 1);
    put_be32(data, 28, 3);
    std::size_t h = 100;
    data[h] = sf::detail::INTERIOR_INDEX;
    put_be16(data, h + 3, 1);
    put_be32(data, h + 8, 3);
    std::vector<std::uint8_t> cell = {0, 0, 0, 2}; // left child = 2
    sf::varint::write(1, cell);                    // divider len
    cell.push_back(0x50);
    std::size_t cell_off = ps - cell.size();
    std::memcpy(data.data() + cell_off, cell.data(), cell.size());
    put_be16(data, h + 12, static_cast<std::uint16_t>(cell_off));
    struct L { std::uint32_t pn; std::uint8_t rec; };
    L leaves[] = {{2, 0x20}, {3, 0x80}};
    for (const L& l : leaves) {
        std::size_t base = (l.pn - 1) * ps;
        data[base] = sf::detail::LEAF_INDEX;
        put_be16(data, base + 3, 1);
        std::vector<std::uint8_t> c;
        sf::varint::write(1, c);
        c.push_back(l.rec);
        std::size_t top = ps - c.size();
        std::memcpy(data.data() + base + top, c.data(), c.size());
        put_be16(data, base + 8, static_cast<std::uint16_t>(top));
    }
    auto hi = sf::Pager::open(data);
    auto giv = sf::walk_index(hi.second, hi.first, 1);
    std::sort(giv.begin(), giv.end());
    std::vector<std::vector<std::uint8_t>> wiv = {{0x20}, {0x50}, {0x80}};
    ISO_CHECK(giv == wiv);

    // wrong page type for index walk
    auto wt = one_leaf_index_db(512, {{0x01}});
    wt[100] = sf::detail::LEAF_TABLE;
    auto hw = sf::Pager::open(wt);
    ISO_CHECK(throws_code(sf::Error::Corrupt, [&] { sf::walk_index(hw.second, hw.first, 1); }));
}

} // namespace

int main() {
    test_varint_golden();
    test_varint_max_u64();
    test_varint_sweep_and_truncation();
    test_record_decode();
    test_header();
    test_pager();
    test_btree_leaf();
    test_btree_interior_and_errors();
    test_btree_overflow();
    test_amplification_guard();
    test_btree_index();
    ISO_TEST_RESULT();
}
