// SpreadsheetModel.cpp — implementation of the engine-backed Qt host model.
//
// Every method here is glue: it turns a QString into the `const char *` the C
// ABI wants, calls the engine, and (for reads) parses the returned JSON into
// the display text a spreadsheet cell should show. The JSON shape is the
// contract shared with the TypeScript and WASM engines:
//
//   {"kind":"number","value":46.0} | {"kind":"text","value":"x"} |
//   {"kind":"boolean","value":true} | {"kind":"empty"} |
//   {"code":"#DIV/0!","kind":"error"}

#include "SpreadsheetModel.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QByteArray>
#include <QFile>
#include <QUrl>

#include <cmath>

// The hand-written C ABI over the Rust engine. The header is vendored next to
// the static library in Vendor/ by scripts/build.sh, and the build (qmake/CMake)
// adds Vendor/ to the include path.
#include "spreadsheet.h"

namespace {

// Consume a `char *` the C ABI returned, as a QString, freeing it with the
// engine's own allocator (sc_string_free — NOT C free(); they may differ). A
// NULL return (e.g. a NULL session) becomes an empty string.
QString takeString(char *p) {
    if (!p) return QString();
    QString s = QString::fromUtf8(p);
    sc_string_free(p);
    return s;
}

// Map one decoded value object (`{"kind":...}`) to the string a spreadsheet cell
// should show. Shared by `display` (one cell) and `window` (a whole rectangle).
QString displayValue(const QJsonObject &obj) {
    const QString kind = obj.value(QStringLiteral("kind")).toString();
    if (kind == QLatin1String("empty")) return QString();
    if (kind == QLatin1String("number")) {
        const double n = obj.value(QStringLiteral("value")).toDouble();
        if (n == std::floor(n) && std::abs(n) < 1e15) {
            return QString::number(static_cast<long long>(n));
        }
        return QString::number(n);
    }
    if (kind == QLatin1String("text")) {
        return obj.value(QStringLiteral("value")).toString();
    }
    if (kind == QLatin1String("boolean")) {
        return obj.value(QStringLiteral("value")).toBool() ? QStringLiteral("TRUE")
                                                           : QStringLiteral("FALSE");
    }
    if (kind == QLatin1String("error")) {
        return obj.value(QStringLiteral("code")).toString(QStringLiteral("#ERR"));
    }
    return QString();
}

} // namespace

SpreadsheetModel::SpreadsheetModel(QObject *parent)
    : QObject(parent), session_(sc_session_new()) {
    seed();
    recompute();
    computeExtent();
    selectInf(1, 1); // prime the infinite-view selection/formula bar at A1
}

SpreadsheetModel::~SpreadsheetModel() {
    sc_session_free(session_);
}

QString SpreadsheetModel::address(int r, int c) {
    // c is 1..5 → 'A'..'E'; r is 0-based → 1-based row number.
    const QChar letter(QLatin1Char(static_cast<char>('A' + (c - 1))));
    return QString(letter) + QString::number(r + 1);
}

void SpreadsheetModel::seed() {
    // The classic cross-footing budget — identical to Engine.swift's seed():
    // column E totals each row, row 5 totals each column, E5 is the grand total.
    // Every total is a formula, so editing any input ripples through on recompute.
    static const struct {
        const char *a1;
        const char *raw;
    } cells[] = {
        {"A1", "15"}, {"B1", "3"},  {"C1", "12"}, {"D1", "8"},  {"E1", "=SUM(A1:D1)"},
        {"A2", "8"},  {"B2", "14"}, {"C2", "7"},  {"D2", "22"}, {"E2", "=SUM(A2:D2)"},
        {"A3", "12"}, {"B3", "9"},  {"C3", "18"}, {"D3", "6"},  {"E3", "=SUM(A3:D3)"},
        {"A4", "4"},  {"B4", "11"}, {"C4", "3"},  {"D4", "17"}, {"E4", "=SUM(A4:D4)"},
        {"A5", "=SUM(A1:A4)"}, {"B5", "=SUM(B1:B4)"}, {"C5", "=SUM(C1:C4)"},
        {"D5", "=SUM(D1:D4)"}, {"E5", "=SUM(E1:E4)"},
        // Far-flung, sparse cells so the infinite view has something to scroll to
        // (the 5×5 parity grid only ever shows A1:E5, so these don't affect it).
        {"Z1000", "=SUM(A1:A4)"},                    // row 1000, col 26: 39
        {"BA50", "far cell"}, {"BB50", "=Z1000*2"},  // row 50, col 53/54: 78
    };
    for (const auto &cell : cells) {
        takeString(sc_set_cell(session_, cell.a1, cell.raw));
    }

    // Attach Excel-style format codes so the engine's display path is visible in
    // the infinite view (which now renders via sc_get_display_window): the
    // cross-foot totals read with thousands grouping + two decimals, and the
    // far-flung Z1000 total as a percent. Values are unchanged — only how the
    // display strings render. Identical to the web demo's seeded formats.
    static const struct {
        const char *a1;
        const char *code;
    } formats[] = {
        {"E1", "#,##0.00"}, {"E2", "#,##0.00"}, {"E3", "#,##0.00"},
        {"E4", "#,##0.00"}, {"E5", "#,##0.00"},
        {"A5", "#,##0.00"}, {"B5", "#,##0.00"}, {"C5", "#,##0.00"}, {"D5", "#,##0.00"},
        {"Z1000", "0.0%"}, // 39 → "3900.0%": proves the format applies far off-origin
    };
    for (const auto &f : formats) {
        sc_set_format(session_, f.a1, f.code);
    }

    // Seed a second "Summary" sheet so cross-sheet references are demoable out of
    // the box: type =Summary!A3 on Sheet1 and it tracks Summary's running total,
    // recomputing live when Summary changes. add_sheet makes it active, so switch
    // back to Sheet1 (index 0) as the default active sheet.
    sc_add_sheet(session_, "Summary");
    takeString(sc_set_cell(session_, "A1", "100"));
    takeString(sc_set_cell(session_, "A2", "200"));
    takeString(sc_set_cell(session_, "A3", "=A1+A2")); // 300
    sc_set_active_sheet(session_, 0);
}

QString SpreadsheetModel::display(const QString &a1) const {
    const QByteArray a1Utf8 = a1.toUtf8();
    const QString json = takeString(sc_get_value(session_, a1Utf8.constData()));

    QJsonParseError err{};
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError || !doc.isObject()) return QString();
    return displayValue(doc.object());
}

QString SpreadsheetModel::valueJson(const QString &a1) const {
    const QByteArray a1Utf8 = a1.toUtf8();
    return takeString(sc_get_value(session_, a1Utf8.constData()));
}

void SpreadsheetModel::recompute() {
    // Rebuild the display matrix: each row is [rowLabel, A, B, C, D, E].
    QVariantList matrix;
    for (int r = 0; r < Rows; ++r) {
        QVariantList row;
        row.append(QString::number(r + 1)); // the row-label gutter
        for (int c = 1; c <= Cols; ++c) {
            row.append(display(address(r, c)));
        }
        matrix.append(QVariant(row));
    }
    viewportRows_ = matrix;
    emit changed();
}

QString SpreadsheetModel::cellAddress() const {
    if (selectedCol_ < 1) return QString::number(selectedRow_ + 1);
    return address(selectedRow_, selectedCol_);
}

QString SpreadsheetModel::selectedRaw() const {
    if (selectedCol_ < 1) return QString();
    const QString a1 = address(selectedRow_, selectedCol_);
    const QByteArray a1Utf8 = a1.toUtf8();
    return takeString(sc_get_raw(session_, a1Utf8.constData()));
}

void SpreadsheetModel::select(int row, int col) {
    selectedRow_ = std::max(0, std::min(Rows - 1, row));
    selectedCol_ = std::max(1, std::min(Cols, col));
    emit selectionChanged();
}

// ── Viewport primitive (virtualized infinite sheet) ──────────────────
// These mirror the engine's get_window / used_range / changed_since reads so a
// windowed QML grid can render only the visible rectangle of an unbounded sheet
// — the Qt sibling of the SwiftUI/web infinite views. 1-based inclusive coords.

QVariantList SpreadsheetModel::window(int row0, int col0, int row1, int col1) const {
    // sc_get_display_window returns each cell already rendered through its format
    // code as a display STRING (the format-aware sibling of sc_get_window), so the
    // QML grid paints the strings directly and never re-derives number formatting.
    // The JSON is {"row0":..,"cols":..,"cells":[["1,234.50",..],..]} (empty cells
    // ""), or {"error":".."} on a bad/oversized request.
    const QString json = takeString(sc_get_display_window(
        session_, static_cast<quint32>(row0), static_cast<quint32>(col0),
        static_cast<quint32>(row1), static_cast<quint32>(col1)));
    QVariantList rows;
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isObject()) return rows; // bad/oversized request → empty
    const QJsonArray cells = doc.object().value(QStringLiteral("cells")).toArray();
    for (const QJsonValue &rowVal : cells) {
        QVariantList row;
        for (const QJsonValue &cell : rowVal.toArray()) {
            row.append(cell.toString());
        }
        rows.append(QVariant(row));
    }
    return rows;
}

QVariantMap SpreadsheetModel::usedRange() const {
    const QString json = takeString(sc_used_range(session_));
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    QVariantMap out;
    if (!doc.isObject()) return out; // "null" (empty sheet) → empty map
    const QJsonObject obj = doc.object();
    out.insert(QStringLiteral("minRow"), obj.value(QStringLiteral("minRow")).toInt());
    out.insert(QStringLiteral("minCol"), obj.value(QStringLiteral("minCol")).toInt());
    out.insert(QStringLiteral("maxRow"), obj.value(QStringLiteral("maxRow")).toInt());
    out.insert(QStringLiteral("maxCol"), obj.value(QStringLiteral("maxCol")).toInt());
    return out;
}

QString SpreadsheetModel::columnLetters(int index) const {
    return takeString(sc_column_letters(session_, static_cast<quint32>(index)));
}

quint64 SpreadsheetModel::currentRevision() const {
    return sc_current_revision(session_);
}

QStringList SpreadsheetModel::changedSince(quint64 since) const {
    const QString json = takeString(sc_changed_since(session_, since));
    QStringList out;
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isObject()) return out;
    for (const QJsonValue &v : doc.object().value(QStringLiteral("changed")).toArray()) {
        out.append(v.toString());
    }
    return out;
}

void SpreadsheetModel::setSelected(const QString &raw) {
    if (selectedCol_ < 1) return;
    setCell(address(selectedRow_, selectedCol_), raw);
}

void SpreadsheetModel::setCell(const QString &a1, const QString &raw) {
    const QByteArray a1Utf8 = a1.toUtf8();
    const QByteArray rawUtf8 = raw.toUtf8();
    takeString(sc_set_cell(session_, a1Utf8.constData(), rawUtf8.constData()));
    recompute();
}

// ── Infinite-sheet view (InfiniteSheet.qml) ──────────────────────────
// The Qt sibling of the SwiftUI InfiniteGridView / web infinite.html. A
// virtualized QML ListView renders only the visible rows; each visible delegate
// asks `rowCells(row)` for that row's display strings (one engine `get_window`
// over a 1×totalCols strip), so an unbounded sheet costs only what's on screen.

QString SpreadsheetModel::rawAt(const QString &a1) const {
    const QByteArray a1Utf8 = a1.toUtf8();
    return takeString(sc_get_raw(session_, a1Utf8.constData()));
}

QString SpreadsheetModel::infAddress() const {
    return columnLetters(infCol_) + QString::number(infRow_);
}

// One row of the infinite view: columns 1..totalCols_ as display strings. The
// engine read is a single-row window; we return its first (only) row, or an
// empty list if the request was rejected/oversized.
QVariantList SpreadsheetModel::rowCells(int row) const {
    if (row < 1) return QVariantList();
    const QVariantList rows = window(row, 1, row, totalCols_);
    if (rows.isEmpty()) return QVariantList();
    return rows.first().toList();
}

// Re-derive the virtual grid size from the engine's data extent plus a margin so
// you can scroll past the data into blank space. Mirrors WindowedSheetModel.resize().
void SpreadsheetModel::computeExtent() {
    const QVariantMap u = usedRange();
    const int maxRow = u.value(QStringLiteral("maxRow"), 1).toInt();
    const int maxCol = u.value(QStringLiteral("maxCol"), 1).toInt();
    totalRows_ = std::max(maxRow + 200, 1000);
    totalCols_ = std::max(maxCol + 30, 60);
    emit extentChanged();
}

// Move the infinite-view selection (clamped to the virtual grid; col/row ≥ 1)
// and pull the selected cell's raw source into the formula bar.
void SpreadsheetModel::selectInf(int row, int col) {
    infRow_ = std::max(1, std::min(totalRows_, row));
    infCol_ = std::max(1, std::min(totalCols_, col));
    infFormula_ = rawAt(infAddress());
    emit infSelectionChanged();
}

// Commit the formula bar into the selected infinite-view cell: write through to
// the engine (which recomputes every dependent), grow the extent if the edit
// reached new ground, re-read the source, and bump `revision` so the visible
// ListView delegates re-fetch their rows.
void SpreadsheetModel::commitInf(const QString &raw) {
    const QString a1 = infAddress();
    const QByteArray a1Utf8 = a1.toUtf8();
    const QByteArray rawUtf8 = raw.toUtf8();
    takeString(sc_set_cell(session_, a1Utf8.constData(), rawUtf8.constData()));
    recompute();        // keep the 5×5 parity grid in sync too
    computeExtent();
    infFormula_ = rawAt(a1);
    revision_++;
    emit infSelectionChanged();
    emit revisionChanged();
}

// Drag-fill: the engine replicates the `src` cell across the inclusive A1
// rectangle (relative refs shift per target, absolute ($) refs pin, the format
// carries). sc_fill returns void; we then recompute the parity grid, regrow the
// extent if the fill reached new ground, and bump `revision` so the visible rows
// re-fetch. A malformed address is a no-op inside the engine.
void SpreadsheetModel::fill(const QString &src, const QString &dstStart, const QString &dstEnd) {
    const QByteArray s = src.toUtf8();
    const QByteArray ds = dstStart.toUtf8();
    const QByteArray de = dstEnd.toUtf8();
    sc_fill(session_, s.constData(), ds.constData(), de.constData());
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
}

// Number formatting: attach an Excel-style format code to the selected cell. The
// code is display-only — the stored value is unchanged; the engine renders it
// through the code (sc_get_display_window). An empty code clears the format. The
// QByteArray locals are NAMED so the UTF-8 buffers outlive the C call.
void SpreadsheetModel::setFormatInf(const QString &code) {
    const QByteArray a1 = infAddress().toUtf8();
    const QByteArray c = code.toUtf8();
    sc_set_format(session_, a1.constData(), c.constData());
    recompute();
    revision_++;
    emit changed();
    emit revisionChanged();
}

// Range sort: the engine reorders the rows of the rectangle by the computed
// values in `keyCol`, moving each row as a record (moved formulas' relative refs
// shift with their row; absolute pin; formats ride along) — sc_sort_range
// returns 1 when applied (or already sorted), 0 for a no-op. The QByteArray
// locals are NAMED so the UTF-8 buffers outlive the C call, and `keyCol` is
// clamped to >= 0 before the unsigned cast (same guard the structural edits use).
bool SpreadsheetModel::sortRange(const QString &start, const QString &end, int keyCol, bool ascending) {
    const QByteArray s = start.toUtf8();
    const QByteArray e = end.toUtf8();
    const uint32_t key = static_cast<uint32_t>(keyCol < 0 ? 0 : keyCol);
    const int ok = sc_sort_range(session_, s.constData(), e.constData(), key, ascending ? 1 : 0);
    recompute();        // keep the 5×5 parity grid in sync too
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
    return ok != 0;
}

// Find: the engine returns {"matches":["A1",…]} (the A1 addresses whose text
// contains the query); parse it into a QStringList. NAMED QByteArray local so the
// UTF-8 buffer outlives the C call. Read-only — no recompute.
QStringList SpreadsheetModel::findMatches(const QString &query, bool inFormulas, bool matchCase) const {
    const QByteArray q = query.toUtf8();
    const QString json = takeString(
        sc_find_all(session_, q.constData(), inFormulas ? 1 : 0, matchCase ? 1 : 0));
    QStringList out;
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isObject()) return out;
    for (const QJsonValue &v : doc.object().value(QStringLiteral("matches")).toArray()) {
        out.append(v.toString());
    }
    return out;
}

// Replace: the engine rewrites `query` → `replacement` in every matching cell's
// source and recomputes; sc_replace_all returns the count changed. NAMED
// QByteArray locals so the UTF-8 buffers outlive the C call.
int SpreadsheetModel::replaceAll(const QString &query, const QString &replacement, bool matchCase) {
    const QByteArray q = query.toUtf8();
    const QByteArray r = replacement.toUtf8();
    const int n = sc_replace_all(session_, q.constData(), r.constData(), matchCase ? 1 : 0);
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
    return n;
}

// Clipboard: copy/cut capture the inclusive rectangle into the engine's
// clipboard; paste places it (the whole block's references shift by the
// destination's offset). The QByteArray locals are NAMED so the UTF-8 buffers
// outlive the C call — a `start.toUtf8().constData()` temporary would dangle.
void SpreadsheetModel::copy(const QString &start, const QString &end) {
    const QByteArray s = start.toUtf8();
    const QByteArray e = end.toUtf8();
    sc_copy(session_, s.constData(), e.constData());
}

void SpreadsheetModel::cut(const QString &start, const QString &end) {
    const QByteArray s = start.toUtf8();
    const QByteArray e = end.toUtf8();
    sc_cut(session_, s.constData(), e.constData());
}

// Returns true when a paste was applied; false (no-op) for an empty clipboard,
// a malformed address, or an off-grid destination. On success, recompute the
// grid, regrow the extent, and bump `revision` so the visible rows re-fetch.
bool SpreadsheetModel::paste(const QString &dstStart) {
    const QByteArray d = dstStart.toUtf8();
    const bool applied = sc_paste(session_, d.constData()) != 0;
    if (applied) {
        recompute();
        computeExtent();
        revision_++;
        emit changed();
        emit revisionChanged();
    }
    return applied;
}

// Structural edits: insert / delete rows or columns at a 1-based position. The
// engine shifts every formula reference at or after the band (a reference whose
// whole band is deleted becomes #REF!) and recomputes; we then rebuild the grid,
// regrow the extent, and bump `revision` so the visible rows re-fetch. `at`/`count`
// arrive as int from QML — clamp the count to ≥ 0 before the u32 cast so a stray
// negative can't wrap to a huge unsigned band.
void SpreadsheetModel::insertRows(int at, int count) {
    sc_insert_rows(session_, static_cast<uint32_t>(at < 0 ? 0 : at),
                   static_cast<uint32_t>(count < 0 ? 0 : count));
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
}

void SpreadsheetModel::deleteRows(int at, int count) {
    sc_delete_rows(session_, static_cast<uint32_t>(at < 0 ? 0 : at),
                   static_cast<uint32_t>(count < 0 ? 0 : count));
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
}

void SpreadsheetModel::insertCols(int at, int count) {
    sc_insert_cols(session_, static_cast<uint32_t>(at < 0 ? 0 : at),
                   static_cast<uint32_t>(count < 0 ? 0 : count));
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
}

void SpreadsheetModel::deleteCols(int at, int count) {
    sc_delete_cols(session_, static_cast<uint32_t>(at < 0 ? 0 : at),
                   static_cast<uint32_t>(count < 0 ? 0 : count));
    recompute();
    computeExtent();
    revision_++;
    emit changed();
    emit revisionChanged();
}

// Save: serialize the whole workbook (source + formats) to a JSON document. The
// host stores the returned string wherever it likes (a file, QSettings, …); the
// engine owns no I/O. takeString frees the C string with the engine's allocator.
QString SpreadsheetModel::serialize() const {
    return takeString(sc_serialize(session_));
}

// Load: replace the workbook from a document produced by serialize(). Returns
// false (workbook untouched) for malformed / unsupported input; on success the
// formulas reload live, so we recompute the grid, regrow the extent, refresh the
// formula bar, and bump `revision` so the visible rows re-fetch.
bool SpreadsheetModel::deserialize(const QString &data) {
    const QByteArray d = data.toUtf8();
    const bool ok = sc_deserialize(session_, d.constData()) != 0;
    if (ok) {
        recompute();
        computeExtent();
        infFormula_ = rawAt(infAddress());
        revision_++;
        emit changed();
        emit infSelectionChanged();
        emit revisionChanged();
    }
    return ok;
}

// ── File open / save — bytes in, bytes out ─────────────────────────────
// A QByteArray carries raw file bytes intact (an .xlsx is a ZIP, an .xls an OLE2
// file) — unlike takeString's char* path, a 0x00 in the middle doesn't truncate
// it. exportBytes copies the engine's (ptr, len) buffer into a QByteArray and
// frees it with sc_bytes_free (the engine's own allocator — NOT free()); a save
// that returns a null pointer / empty doc yields an empty QByteArray. importBytes
// hands the engine a borrowed (constData, size) pair and, on success, refreshes
// the grid exactly as deserialize() does.

QByteArray SpreadsheetModel::exportBytes(const QString &format) const {
    size_t len = 0;
    uint8_t *ptr = nullptr;
    if (format == QLatin1String("xlsx")) ptr = sc_save_xlsx(session_, &len);
    else if (format == QLatin1String("xls")) ptr = sc_save_xls(session_, &len);
    else if (format == QLatin1String("csv")) ptr = sc_save_csv(session_, &len);
    else if (format == QLatin1String("tsv")) ptr = sc_save_tsv(session_, &len);
    else if (format == QLatin1String("json")) ptr = sc_save_json(session_, &len);
    else return QByteArray();
    if (ptr == nullptr) return QByteArray();
    // Copy the bytes out BEFORE releasing the engine's buffer with its allocator.
    // Use qsizetype (64-bit in Qt 6) for the length so a large file can't wrap a
    // narrower int.
    QByteArray bytes(reinterpret_cast<const char *>(ptr), static_cast<qsizetype>(len));
    sc_bytes_free(ptr, len);
    return bytes;
}

bool SpreadsheetModel::importBytes(const QString &format, const QByteArray &bytes) {
    if (bytes.isEmpty()) return false; // empty input is a safe no-op
    const auto *p = reinterpret_cast<const uint8_t *>(bytes.constData());
    const auto n = static_cast<size_t>(bytes.size());
    int ok = 0;
    if (format == QLatin1String("xlsx")) ok = sc_load_xlsx(session_, p, n);
    else if (format == QLatin1String("xls")) ok = sc_load_xls(session_, p, n);
    else if (format == QLatin1String("csv")) ok = sc_load_csv(session_, p, n);
    else if (format == QLatin1String("tsv")) ok = sc_load_tsv(session_, p, n);
    else if (format == QLatin1String("json")) ok = sc_load_json(session_, p, n);
    else return false;
    if (ok) {
        recompute();
        computeExtent();
        infFormula_ = rawAt(infAddress());
        revision_++;
        emit changed();
        emit infSelectionChanged();
        emit revisionChanged();
    }
    return ok != 0;
}

// saveFile/openFile wrap the codec core with QFile so QML's FileDialog can hand a
// real path (a proper OS Save/Open dialog — no fixed demo path needed here, unlike
// the plugin-free hosts). QML's FileDialog reports a `file://` URL, so accept both
// a local path and a URL (normalising through QUrl). A failed file open, or a codec
// that rejects the bytes, returns false and leaves the workbook untouched.
static QString localPathOf(const QString &pathOrUrl) {
    const QUrl url(pathOrUrl);
    return url.isLocalFile() ? url.toLocalFile() : pathOrUrl;
}

bool SpreadsheetModel::saveFile(const QString &path, const QString &format) {
    const QByteArray bytes = exportBytes(format);
    QFile file(localPathOf(path));
    if (!file.open(QIODevice::WriteOnly)) return false;
    const bool ok = file.write(bytes) == bytes.size();
    file.close();
    return ok;
}

bool SpreadsheetModel::openFile(const QString &path, const QString &format) {
    QFile file(localPathOf(path));
    if (!file.open(QIODevice::ReadOnly)) return false;
    const QByteArray bytes = file.readAll();
    file.close();
    return importBytes(format, bytes);
}

// Undo / redo availability — thin reads of the engine's history stacks; the
// canUndo/canRedo Q_PROPERTYs are bound to revisionChanged so the QML buttons
// re-evaluate these after every edit (and after undo/redo themselves).
bool SpreadsheetModel::canUndo() const { return sc_can_undo(session_) != 0; }
bool SpreadsheetModel::canRedo() const { return sc_can_redo(session_) != 0; }

// Undo / redo: revert / replay the most recent edit. On success (returned true)
// any cell could have changed, so we recompute the grid, regrow the extent,
// refresh the formula bar, and bump `revision` — which also re-evaluates the
// canUndo/canRedo bindings. Shared body for both directions.
bool SpreadsheetModel::undo() {
    const bool ok = sc_undo(session_) != 0;
    if (ok) {
        recompute();
        computeExtent();
        infFormula_ = rawAt(infAddress());
        revision_++;
        emit changed();
        emit infSelectionChanged();
        emit revisionChanged();
    }
    return ok;
}

bool SpreadsheetModel::redo() {
    const bool ok = sc_redo(session_) != 0;
    if (ok) {
        recompute();
        computeExtent();
        infFormula_ = rawAt(infAddress());
        revision_++;
        emit changed();
        emit infSelectionChanged();
        emit revisionChanged();
    }
    return ok;
}

// ── Multi-sheet workbook ─────────────────────────────────────────────

// Shared tail after a sheet op that changed the active sheet (add/select/rename/
// delete): reset the infinite-view selection to the active sheet's top-left,
// re-read the grid, resync the formula bar, regrow the extent, bump the revision,
// and fire the rebind signals (grid + tab bar).
void SpreadsheetModel::refreshAfterSheetOp() {
    infRow_ = 1;
    infCol_ = 1;
    recompute();
    computeExtent();
    infFormula_ = rawAt(infAddress());
    revision_++;
    emit changed();
    emit infSelectionChanged();
    emit revisionChanged();
    emit sheetsChanged();
}

QVariantMap SpreadsheetModel::sheetNames() const {
    QVariantMap out;
    const QString json = takeString(sc_sheet_names(session_));
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isObject()) return out;
    const QJsonObject obj = doc.object();
    QStringList names;
    for (const QJsonValue &v : obj.value(QStringLiteral("sheets")).toArray()) {
        names.append(v.toString());
    }
    out.insert(QStringLiteral("sheets"), names);
    out.insert(QStringLiteral("active"), obj.value(QStringLiteral("active")).toInt());
    return out;
}

int SpreadsheetModel::activeSheet() const {
    return static_cast<int>(sc_active_sheet(session_));
}

bool SpreadsheetModel::selectSheet(int index) {
    if (sc_set_active_sheet(session_, static_cast<unsigned>(std::max(0, index))) == 0)
        return false;
    refreshAfterSheetOp();
    return true;
}

bool SpreadsheetModel::addSheet(const QString &name) {
    const QByteArray n = name.toUtf8();
    if (sc_add_sheet(session_, n.constData()) == 0) return false;
    refreshAfterSheetOp();
    return true;
}

bool SpreadsheetModel::renameSheet(int index, const QString &newName) {
    const QByteArray n = newName.toUtf8();
    if (sc_rename_sheet(session_, static_cast<unsigned>(std::max(0, index)), n.constData()) == 0)
        return false;
    refreshAfterSheetOp();
    return true;
}

bool SpreadsheetModel::deleteSheet(int index) {
    if (sc_delete_sheet(session_, static_cast<unsigned>(std::max(0, index))) == 0)
        return false;
    refreshAfterSheetOp();
    return true;
}
