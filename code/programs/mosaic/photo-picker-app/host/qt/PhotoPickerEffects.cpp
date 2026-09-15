#include "PhotoPickerEffects.h"

#include <QByteArray>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QMap>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariant>
#include <QVariantMap>
#include <QtGlobal>

#include <exception>

#include "MosaicHost.h"

namespace {

// The three tagged outcomes UI59 SS3 defines. Cancellation is not a failure:
// Escape in a file dialog is an ordinary thing for a person to do.
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

// Common photo MIME types this app's own request asks for (see
// photo-picker-mosaic-app's ACCEPT_IMAGE_TYPES) mapped to the file extensions
// Qt's filter syntax actually wants -- QFileDialog filters by extension glob,
// not MIME type, so the handler owns this table rather than the app needing
// to know Qt-specific filter syntax (UI59 SS3), matching the table the XAML
// handler carries for the same reason.
const QMap<QString, QStringList> &mimeTypeExtensions()
{
    static const QMap<QString, QStringList> table{
        {QStringLiteral("image/jpeg"), {QStringLiteral(".jpg"), QStringLiteral(".jpeg")}},
        {QStringLiteral("image/png"), {QStringLiteral(".png")}},
        {QStringLiteral("image/webp"), {QStringLiteral(".webp")}},
        {QStringLiteral("image/gif"), {QStringLiteral(".gif")}},
        {QStringLiteral("image/bmp"), {QStringLiteral(".bmp")}},
        {QStringLiteral("image/tiff"), {QStringLiteral(".tif"), QStringLiteral(".tiff")}},
    };
    return table;
}

// The reverse of the table above, for reporting the picked file's MIME type
// back to the app (UI59 SS3's `mimeType` result field) -- a picked path
// carries no MIME type of its own, so this is recovered from the extension.
QString mimeTypeForExtension(const QString &extension)
{
    const QString lowered = extension.toLower();
    for (auto it = mimeTypeExtensions().constBegin(); it != mimeTypeExtensions().constEnd(); ++it) {
        for (const QString &candidate : it.value()) {
            if (candidate == lowered) {
                return it.key();
            }
        }
    }
    return QStringLiteral("application/octet-stream");
}

// Qt's filter syntax is "Description (*.ext1 *.ext2)", a single parenthesized
// group of space-separated globs -- distinct from XAML's per-extension
// FileTypeFilter list, so this is built fresh rather than shared with it.
// An `accept` MIME type this host doesn't recognise is dropped rather than
// failing the request (UI59 SS3); if nothing was recognised, "*" (any file).
QString fileFilter(const QVariant &payload)
{
    QStringList globs;
    const auto accept = payload.toMap().value(QStringLiteral("accept")).toStringList();
    for (const QString &mimeType : accept) {
        const auto extensions = mimeTypeExtensions().value(mimeType);
        for (const QString &extension : extensions) {
            globs << QStringLiteral("*%1").arg(extension);
        }
    }
    if (globs.isEmpty()) {
        globs << QStringLiteral("*");
    }
    return QStringLiteral("Files (%1)").arg(globs.join(QLatin1Char(' ')));
}

// A picked file is read fully into memory and base64-encoded (UI59 SS3's
// `bytes` field is the whole file); without a cap, a caller picking an
// arbitrarily large file costs an arbitrarily large amount of host memory.
// 50 MiB matches the XAML handler's own cap, for parity across backends.
constexpr qint64 MaxPickedFileBytes = 50LL * 1024 * 1024;

// Reads at most `MaxPickedFileBytes` and reports whether the file ran over,
// rather than trusting `QFileInfo::size()` checked once beforehand -- the
// XAML handler's first cut made exactly that mistake (a pre-check alone is
// TOCTOU: the file can grow between the check and the read), caught by
// `/security-review` on that PR before it shipped. Reading in bounded chunks
// here means this handler never has that gap to begin with.
bool readBounded(QFile &file, QByteArray &out)
{
    out.clear();
    QByteArray chunk;
    chunk.resize(64 * 1024);
    while (true) {
        const qint64 got = file.read(chunk.data(), chunk.size());
        if (got < 0) {
            return false;
        }
        if (got == 0) {
            return true;
        }
        if (out.size() + got > MaxPickedFileBytes) {
            return false;
        }
        out.append(chunk.constData(), static_cast<int>(got));
    }
}

void handlePickPhoto(MosaicHost &host, const QVariant &effectId, const QVariant &payload)
{
    const QString path = QFileDialog::getOpenFileName(
        nullptr,
        QObject::tr("Pick a Photo"),
        QDir::homePath(),
        fileFilter(payload));
    if (path.isEmpty()) {
        host.completeEffect(effectId, cancelledOutcome());
        return;
    }

    // Sized up before it is opened, as a fast-fail for the common case --
    // `readBounded` below enforces the real limit regardless (see its own
    // comment), so this is a courtesy, not the only guard.
    const QFileInfo info(path);
    if (!info.isFile()) {
        host.completeEffect(effectId, failedOutcome(QObject::tr("that is not a regular file")));
        return;
    }

    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        // Not `file.errorString()` -- a raw Qt I/O error string is host data
        // this handler does not control the shape of, and `failed.message` is
        // app-visible (the same reasoning that moved the XAML handler off raw
        // exception messages after `/security-review`, UI59 SS4.2). A picked
        // file living on the user's own machine is low-severity here, but the
        // README already asks future apps to copy this handler near-verbatim,
        // so the safer default is the one worth setting.
        host.completeEffect(effectId, failedOutcome(QObject::tr("couldn't read the selected file")));
        return;
    }

    QByteArray data;
    const bool ok = readBounded(file, data);
    file.close();
    if (!ok) {
        host.completeEffect(
            effectId,
            failedOutcome(QObject::tr(
                "the selected file is too large or couldn't be read (limit is %1 bytes)")
                              .arg(MaxPickedFileBytes)));
        return;
    }

    host.completeEffect(
        effectId,
        okOutcome({
            {QStringLiteral("name"), info.fileName()},
            {QStringLiteral("mimeType"), mimeTypeForExtension(QStringLiteral(".%1").arg(info.suffix()))},
            {QStringLiteral("bytes"), QString::fromLatin1(data.toBase64())},
        }));
}

} // namespace

void installPhotoPickerEffects(MosaicHost &host)
{
    QObject::connect(
        &host,
        &MosaicHost::effectRequested,
        &host,
        [&host](const QVariant &effectId,
                const QString &kind,
                const QVariant &payload,
                const QString &delivery) {
            if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) {
                return;
            }
            if (kind != QStringLiteral("files.open")) {
                // An awaited kind this host does not know is left alone on
                // purpose -- the host's own sweep fails it with a reason the
                // application shows (same convention as Engram's Qt handler).
                return;
            }
            // Nothing may escape into the emit -- an exception here unwinds
            // past `settleEffects`, `failOutstanding` never runs, and the id
            // stays `awaiting_` for the life of the process. Answering with
            // the reason is strictly better (same reasoning as Engram's Qt
            // handler, UI47's worked example).
            try {
                handlePickPhoto(host, effectId, payload);
            } catch (const std::exception &) {
                host.completeEffect(effectId, failedOutcome(QObject::tr("couldn't pick a photo")));
            } catch (...) {
                host.completeEffect(effectId, failedOutcome(QObject::tr("couldn't pick a photo")));
            }
        },
        // Direct, not queued: the handler runs inline (QFileDialog::
        // getOpenFileName is a blocking static call, same as Engram's Qt
        // import/export), so the effect is answered before this lambda
        // returns. A queued connection would return immediately and the
        // host's sweep would fail the effect as unanswered before the dialog
        // even opened.
        Qt::DirectConnection);
}
