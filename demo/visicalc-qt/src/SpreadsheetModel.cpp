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
    };
    for (const auto &cell : cells) {
        takeString(sc_set_cell(session_, cell.a1, cell.raw));
    }
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
    const QString json = takeString(sc_get_window(
        session_, static_cast<quint32>(row0), static_cast<quint32>(col0),
        static_cast<quint32>(row1), static_cast<quint32>(col1)));
    QVariantList rows;
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isObject()) return rows; // bad/oversized request → empty
    const QJsonArray values = doc.object().value(QStringLiteral("values")).toArray();
    for (const QJsonValue &rowVal : values) {
        QVariantList row;
        for (const QJsonValue &cell : rowVal.toArray()) {
            row.append(displayValue(cell.toObject()));
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
