#include "engram_effects.h"

#include <QByteArray>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QRegularExpression>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantMap>
#include <QtGlobal>

#include <exception>

#include "MosaicHost.h"

namespace {

// The three tagged outcomes the protocol defines. Cancellation is not a
// failure: Escape in a file dialog is an ordinary thing for a person to do, and
// the application says "Import cancelled." rather than "Import failed."
QVariantMap okOutcome(const QVariantMap &value)
{
    return QVariantMap{{QStringLiteral("ok"), value}};
}

QVariantMap cancelledOutcome()
{
    return QVariantMap{{QStringLiteral("cancelled"), QVariantMap{}}};
}

QVariantMap failedOutcome(const QString &message)
{
    return QVariantMap{
        {QStringLiteral("failed"),
         QVariantMap{{QStringLiteral("message"), message}}}};
}

// The largest package this host will read into memory.
//
// Aligned with what the engine will actually accept: the package layer refuses
// a collection past 256 MiB on native targets, so a larger file cannot import
// whatever this does. Checked BEFORE `readAll`, because the point is to never
// make the allocation -- one import costs roughly seven times the file's size
// in flight (the bytes, their base64, that base64 as UTF-16, the JSON envelope,
// and the runtime's own copy), and the application's own cap sits at the far
// end of all of it.
constexpr qint64 MaxImportBytes = 256LL * 1024 * 1024;

// `*.apkg *.colpkg` from the extensions the application sent, so the picker
// shows what this build can actually read rather than a hardcoded guess that
// drifts from the engine.
QString fileFilter(const QVariant &payload, const QString &key, const QStringList &fallback)
{
    QStringList extensions = fallback;
    const auto declared = payload.toMap().value(key).toStringList();
    if (!declared.isEmpty()) {
        extensions = declared;
    }
    // `)` and `;;` are structural in a Qt filter and `*?[` are globs, so an
    // extension is accepted only if it looks like one. The payload comes from
    // this application rather than a user today, but a filter that can be split
    // in two or widened to everything is not something to leave to that staying
    // true.
    static const QRegularExpression shape(QStringLiteral("^\\.?[A-Za-z0-9_-]{1,16}$"));
    QStringList globs;
    globs.reserve(extensions.size());
    for (const QString &extension : extensions) {
        if (!shape.match(extension).hasMatch()) {
            continue;
        }
        globs << QStringLiteral("*%1").arg(
            extension.startsWith('.') ? extension : QStringLiteral(".%1").arg(extension));
    }
    if (globs.isEmpty()) {
        // Everything was refused, so fall back rather than emit `(...)` with an
        // empty glob list, which Qt reads as "match nothing".
        for (const QString &extension : fallback) {
            globs << QStringLiteral("*%1").arg(extension);
        }
    }
    return QStringLiteral("Anki packages (%1)").arg(globs.join(QLatin1Char(' ')));
}

// The bytes travel, not the path.
//
// The application built this package and sent it out in the payload, so the
// host only has to write it. That is the arrangement sandboxing forces: on
// macOS a process may open only what the user picked in the host's own dialog,
// so the file writing has to happen here, on this side of that dialog.
void handleExport(MosaicHost &host, const QVariant &effectId, const QVariant &payload)
{
    // Forced to a bare filename. `QDir::filePath` happily joins a relative path
    // with separators, so a suggestion of `../.ssh/authorized_keys` would open
    // the dialog in a different directory with only the basename visible --
    // and the retired binding scrubbed separators out of its deck-derived name,
    // so accepting them here would be losing a check rather than never having
    // had one. Nothing sends this key today; that is not a reason to trust it.
    QString suggested =
        QFileInfo(payload.toMap().value(QStringLiteral("suggestedName")).toString())
            .fileName();
    if (suggested == QStringLiteral(".") || suggested == QStringLiteral("..")) {
        suggested.clear();
    }
    QString path = QFileDialog::getSaveFileName(
        nullptr,
        QObject::tr("Export Anki package"),
        QDir::home().filePath(suggested.isEmpty() ? QStringLiteral("engram.apkg") : suggested),
        fileFilter(payload, QStringLiteral("extensions"), {QStringLiteral(".apkg")}));
    if (path.isEmpty()) {
        host.completeEffect(effectId, cancelledOutcome());
        return;
    }
    if (QFileInfo(path).suffix().isEmpty()) {
        path += QStringLiteral(".apkg");
    }

    const QString encoded = payload.toMap().value(QStringLiteral("apkg")).toString();
    if (encoded.isEmpty()) {
        host.completeEffect(
            effectId, failedOutcome(QObject::tr("the export carried no package")));
        return;
    }
    // `AbortOnBase64DecodingErrors` rather than the permissive default: silently
    // discarding a bad character would write a corrupt `.apkg` that only fails
    // later, inside Anki, where nothing points back here.
    const QByteArray decoded = QByteArray::fromBase64(
        encoded.toLatin1(), QByteArray::Base64Encoding | QByteArray::AbortOnBase64DecodingErrors);
    if (decoded.isEmpty()) {
        host.completeEffect(
            effectId, failedOutcome(QObject::tr("the export package was not valid base64")));
        return;
    }

    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        host.completeEffect(effectId, failedOutcome(file.errorString()));
        return;
    }
    if (file.write(decoded) != decoded.size()) {
        const QString message = file.errorString();
        file.close();
        host.completeEffect(effectId, failedOutcome(message));
        return;
    }
    // Closed explicitly and checked: `QFile` flushes on destruction and
    // swallows a failure there, so a full disk would otherwise report success
    // for a truncated file.
    file.close();
    if (file.error() != QFile::NoError) {
        host.completeEffect(effectId, failedOutcome(file.errorString()));
        return;
    }
    host.completeEffect(effectId, okOutcome({}));
}

void handleImport(MosaicHost &host, const QVariant &effectId, const QVariant &payload)
{
    const QString path = QFileDialog::getOpenFileName(
        nullptr,
        QObject::tr("Import Anki package"),
        QDir::homePath(),
        fileFilter(
            payload,
            QStringLiteral("accept"),
            {QStringLiteral(".apkg"), QStringLiteral(".colpkg")}));
    if (path.isEmpty()) {
        host.completeEffect(effectId, cancelledOutcome());
        return;
    }

    // Sized up before it is opened. `readAll` on a fifo or a character device
    // never reaches EOF -- and `QFileInfo::size()` reports 0 for one -- so the
    // regular-file test is doing real work here, not restating the size test.
    const QFileInfo info(path);
    if (!info.isFile()) {
        host.completeEffect(
            effectId, failedOutcome(QObject::tr("that is not a regular file")));
        return;
    }
    if (info.size() <= 0) {
        host.completeEffect(effectId, failedOutcome(QObject::tr("that file is empty")));
        return;
    }
    if (info.size() > MaxImportBytes) {
        host.completeEffect(
            effectId, failedOutcome(QObject::tr("that package is too large to open")));
        return;
    }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        host.completeEffect(effectId, failedOutcome(file.errorString()));
        return;
    }
    const QByteArray data = file.readAll();
    if (data.isEmpty()) {
        host.completeEffect(
            effectId, failedOutcome(QObject::tr("that file is empty")));
        return;
    }
    // The application decodes and merges. Reading the file is the host's whole
    // job here, for the same sandboxing reason the export writes it.
    host.completeEffect(
        effectId,
        okOutcome({{QStringLiteral("apkg"), QString::fromLatin1(data.toBase64())}}));
}

} // namespace

void installEngramEffects(MosaicHost &host)
{
    QObject::connect(
        &host,
        &MosaicHost::effectRequested,
        &host,
        [&host](const QVariant &effectId,
                const QString &kind,
                const QVariant &payload,
                const QString &delivery) {
            // Only the awaited kinds are answered. `openCard` arrives as a
            // `Notify` and is deliberately not handled: nothing is waiting on
            // it, and answering an effect the runtime is not awaiting is
            // refused anyway.
            if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) {
                return;
            }
            // Nothing may escape into the emit.
            //
            // This runs inside the host's `settleEffects`, and the `emit` there
            // has no handler: an exception unwinds past it, `failOutstanding`
            // never runs, and the id stays in `awaiting_` -- so the runtime
            // refuses every later snapshot and restore for the life of the
            // process. That is the silent permanent failure the whole sweep
            // exists to prevent, reached by throwing instead of by forgetting.
            // An exception leaving a directly-connected slot is also undefined
            // behaviour in Qt 6.
            //
            // Answering with the reason is strictly better: the application
            // shows it, the effect is discharged, and persistence survives.
            try {
                if (kind == QStringLiteral("importAnki")) {
                    handleImport(host, effectId, payload);
                } else if (kind == QStringLiteral("exportAnki")) {
                    handleExport(host, effectId, payload);
                }
            } catch (const std::exception &error) {
                host.completeEffect(effectId, failedOutcome(QString::fromUtf8(error.what())));
            } catch (...) {
                host.completeEffect(
                    effectId, failedOutcome(QObject::tr("the file dialog failed")));
            }
            // An awaited kind this host does not know is left alone on purpose.
            // The host's own sweep then fails it with a reason the application
            // shows, which is a better outcome than this file inventing one --
            // and it is what keeps a new effect kind from silently doing
            // nothing while appearing handled.
        },
        // Direct, not queued: the handler runs inside the host's settle, which
        // is where an inline answer belongs. A queued connection would return
        // immediately, the sweep would fail the effect as unanswered, and the
        // dialog's eventual answer would be rejected as already completed.
        Qt::DirectConnection);
}
