#include "MosaicHost.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QStringList>

namespace {
template <typename T>
T resolveSymbol(QLibrary &library, const char *name) {
  return reinterpret_cast<T>(library.resolve(name));
}
}

MosaicHost::MosaicHost(QObject *parent) : QObject(parent) {}

MosaicHost::~MosaicHost() {
  if (session_ != nullptr && sessionFree_ != nullptr) {
    sessionFree_(session_);
  }
}

QVariantMap MosaicHost::props() {
  if (!ensureLoaded()) {
    return {};
  }

  const QByteArray deck = deckId();
  return hostResponseFromJson(takeCString(engramAppProps_(session_, deck.constData(), nowMs())));
}

QVariantMap MosaicHost::handleEvent(const QVariantMap &event) {
  if (!ensureLoaded()) {
    return {};
  }

  const QByteArray eventJson =
      QJsonDocument(QJsonObject::fromVariantMap(event)).toJson(QJsonDocument::Compact);
  const QByteArray deck = deckId();
  const QVariantMap response = hostResponseFromJson(takeCString(
      handleEngramAppEvent_(session_, eventJson.constData(), deck.constData(), nowMs())));
  if (!response.contains(QStringLiteral("error"))) {
    persistSnapshot();
  }
  return handleHostIntent(response);
}

bool MosaicHost::ensureLoaded() {
  if (session_ != nullptr) {
    return true;
  }

  if (!library_.isLoaded()) {
    const QString appDir = QCoreApplication::applicationDirPath();
    const QStringList libraryNames = {
#if defined(Q_OS_WIN)
        QStringLiteral("engram_capi.dll"),
#elif defined(Q_OS_MACOS)
        QStringLiteral("libengram_capi.dylib"),
#else
        QStringLiteral("libengram_capi.so"),
#endif
        QStringLiteral("engram_capi")};

    for (const QString &name : libraryNames) {
      library_.setFileName(QDir(appDir).filePath(name));
      if (library_.load()) {
        break;
      }
      library_.setFileName(name);
      if (library_.load()) {
        break;
      }
    }
  }

  if (!library_.isLoaded()) {
    qWarning() << "Engram MosaicHost could not load engram-capi:" << library_.errorString();
    return false;
  }

  sessionNewDemo_ = resolveSymbol<EgSessionNewDemoFn>(library_, "eg_session_new_demo");
  sessionFree_ = resolveSymbol<EgSessionFreeFn>(library_, "eg_session_free");
  stringFree_ = resolveSymbol<EgStringFreeFn>(library_, "eg_string_free");
  snapshot_ = resolveSymbol<EgSnapshotFn>(library_, "eg_snapshot");
  loadSnapshot_ = resolveSymbol<EgLoadSnapshotFn>(library_, "eg_load_snapshot");
  engramAppProps_ = resolveSymbol<EgEngramAppPropsFn>(library_, "eg_engram_app_props");
  handleEngramAppEvent_ =
      resolveSymbol<EgHandleEngramAppEventFn>(library_, "eg_handle_engram_app_event");
  exportAnkiApkg_ = resolveSymbol<EgExportAnkiApkgFn>(library_, "eg_export_anki_apkg");
  mergeAnkiApkg_ = resolveSymbol<EgMergeAnkiApkgFn>(library_, "eg_merge_anki_apkg");

  if (sessionNewDemo_ == nullptr || sessionFree_ == nullptr || stringFree_ == nullptr ||
      snapshot_ == nullptr || loadSnapshot_ == nullptr || engramAppProps_ == nullptr ||
      handleEngramAppEvent_ == nullptr || exportAnkiApkg_ == nullptr ||
      mergeAnkiApkg_ == nullptr) {
    qWarning() << "Engram MosaicHost loaded engram-capi but required symbols are missing";
    library_.unload();
    return false;
  }

  session_ = sessionNewDemo_();
  if (session_ == nullptr) {
    qWarning() << "Engram MosaicHost failed to create an Engram session";
    return false;
  }
  hydrateSession();
  return true;
}

void MosaicHost::hydrateSession() {
  const QString path = snapshotPath();
  QFile file(path);
  if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
    const QByteArray snapshot = file.readAll();
    const QVariantMap loaded =
        hostResponseFromJson(takeCString(loadSnapshot_(session_, snapshot.constData())));
    if (!loaded.contains(QStringLiteral("error"))) {
      return;
    }
    qWarning() << "Engram MosaicHost persisted snapshot was invalid; using demo state";
  }

  persistSnapshot();
}

void MosaicHost::persistSnapshot() {
  if (session_ == nullptr || snapshot_ == nullptr) {
    return;
  }

  const QString json = takeCString(snapshot_(session_));
  QJsonParseError parseError;
  const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &parseError);
  if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
    qWarning() << "Engram MosaicHost could not parse snapshot JSON:" << parseError.errorString();
    return;
  }

  const QJsonObject root = document.object();
  if (root.value(QStringLiteral("ok")).toBool(true) == false ||
      root.value(QStringLiteral("state")).isUndefined() ||
      root.value(QStringLiteral("state")).isNull()) {
    qWarning() << "Engram MosaicHost snapshot response was not persistable";
    return;
  }

  const QString path = snapshotPath();
  const QFileInfo info(path);
  QDir().mkpath(info.absolutePath());
  QFile file(path);
  if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text)) {
    qWarning() << "Engram MosaicHost could not persist snapshot:" << file.errorString();
    return;
  }

  file.write(QJsonDocument(root.value(QStringLiteral("state")).toObject()).toJson(QJsonDocument::Compact));
}

QString MosaicHost::takeCString(char *value) const {
  if (value == nullptr) {
    return {};
  }
  const QString out = QString::fromUtf8(value);
  if (stringFree_ != nullptr) {
    stringFree_(value);
  }
  return out;
}

QVariantMap MosaicHost::hostResponseFromJson(const QString &json) const {
  if (json.isEmpty()) {
    return {};
  }

  QJsonParseError parseError;
  const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &parseError);
  if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
    qWarning() << "Engram MosaicHost received invalid JSON:" << parseError.errorString();
    return {};
  }

  const QJsonObject root = document.object();
  QVariantMap response;

  if (root.value(QStringLiteral("ok")).toBool(true) == false) {
    response.insert(QStringLiteral("error"), root.value(QStringLiteral("error")).toVariant());
    return response;
  }

  const QJsonObject hostIntent = root.value(QStringLiteral("hostIntent")).toObject();
  if (!hostIntent.isEmpty()) {
    qInfo().noquote() << "Engram host intent:"
                      << QJsonDocument(hostIntent).toJson(QJsonDocument::Compact);
    response.insert(QStringLiteral("hostIntent"), hostIntent.toVariantMap());
  }

  response.insert(
      QStringLiteral("props"),
      camelCaseProps(root.value(QStringLiteral("props")).toObject().toVariantMap()));
  return response;
}

QVariantMap MosaicHost::handleHostIntent(const QVariantMap &response) {
  const QVariantMap hostIntent = response.value(QStringLiteral("hostIntent")).toMap();
  const QString type = hostIntent.value(QStringLiteral("type")).toString();
  if (type == QStringLiteral("importAnki")) {
    return importAnkiPackage(response, hostIntent);
  }
  if (type == QStringLiteral("exportAnki")) {
    return exportAnkiPackage(response, hostIntent);
  }
  return response;
}

QVariantMap MosaicHost::importAnkiPackage(
    const QVariantMap &response,
    const QVariantMap &hostIntent) {
  const QStringList extensions =
      hostIntentExtensions(
          hostIntent,
          QStringLiteral("accept"),
          QStringList{QStringLiteral(".apkg"), QStringLiteral(".colpkg")});
  const QString path = QFileDialog::getOpenFileName(
      nullptr,
      QStringLiteral("Import Anki package"),
      QDir::homePath(),
      ankiFileFilter(extensions));
  if (path.isEmpty()) {
    return hostResultResponse(response, hostIntent, QStringLiteral("cancelled"));
  }

  QFile file(path);
  if (!file.open(QIODevice::ReadOnly)) {
    const QString error = file.errorString();
    qWarning() << "Engram MosaicHost could not read Anki package:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("read-error"), path, error);
  }

  const QByteArray data = file.readAll();
  const QString json = takeCString(mergeAnkiApkg_(
      session_,
      reinterpret_cast<const quint8 *>(data.constData()),
      static_cast<std::size_t>(data.size())));
  const QVariantMap imported = hostResponseFromJson(json);
  if (imported.contains(QStringLiteral("error"))) {
    const QString error = imported.value(QStringLiteral("error")).toString();
    qWarning() << "Engram MosaicHost could not import Anki package:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("import-error"), path, error);
  }

  persistSnapshot();
  QVariantMap refreshed = props();
  refreshed.insert(QStringLiteral("hostIntent"), hostIntent);
  QVariantMap hostResult;
  hostResult.insert(QStringLiteral("status"), QStringLiteral("imported"));
  hostResult.insert(QStringLiteral("path"), path);
  refreshed.insert(QStringLiteral("hostResult"), hostResult);
  return withHostStatusProps(refreshed, hostResult);
}

QVariantMap MosaicHost::exportAnkiPackage(
    const QVariantMap &response,
    const QVariantMap &hostIntent) {
  const QStringList extensions =
      hostIntentExtensions(
          hostIntent,
          QStringLiteral("extensions"),
          QStringList{QStringLiteral(".apkg")});
  QString path = QFileDialog::getSaveFileName(
      nullptr,
      QStringLiteral("Export Anki package"),
      QDir::home().filePath(suggestedAnkiFileName(hostIntent)),
      ankiFileFilter(extensions));
  if (path.isEmpty()) {
    return hostResultResponse(response, hostIntent, QStringLiteral("cancelled"));
  }

  if (QFileInfo(path).suffix().isEmpty()) {
    path += QStringLiteral(".apkg");
  }

  const QString json = takeCString(exportAnkiApkg_(session_));
  QJsonParseError parseError;
  const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &parseError);
  if (parseError.error != QJsonParseError::NoError || !document.isObject()) {
    const QString error = parseError.errorString();
    qWarning() << "Engram MosaicHost could not parse exported Anki package JSON:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("export-error"), path, error);
  }

  const QJsonObject root = document.object();
  if (root.value(QStringLiteral("ok")).toBool(true) == false) {
    const QString error = root.value(QStringLiteral("error")).toString(QStringLiteral("unknown error"));
    qWarning() << "Engram MosaicHost could not export Anki package:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("export-error"), path, error);
  }

  const QByteArray data = jsonByteArray(root, QStringLiteral("apkg"));
  QFile file(path);
  if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
    const QString error = file.errorString();
    qWarning() << "Engram MosaicHost could not save Anki package:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("write-error"), path, error);
  }
  file.write(data);
  file.close();
  if (file.error() != QFile::NoError) {
    const QString error = file.errorString();
    qWarning() << "Engram MosaicHost could not finish saving Anki package:" << error;
    return hostResultResponse(response, hostIntent, QStringLiteral("write-error"), path, error);
  }

  return hostResultResponse(response, hostIntent, QStringLiteral("exported"), path);
}

QVariantMap MosaicHost::hostResultResponse(
    const QVariantMap &response,
    const QVariantMap &hostIntent,
    const QString &status,
    const QString &path,
    const QString &error) const {
  QVariantMap out = response;
  out.insert(QStringLiteral("hostIntent"), hostIntent);
  QVariantMap hostResult;
  hostResult.insert(QStringLiteral("status"), status);
  if (!path.isEmpty()) {
    hostResult.insert(QStringLiteral("path"), path);
  }
  if (!error.isEmpty()) {
    hostResult.insert(QStringLiteral("error"), error);
  }
  out.insert(QStringLiteral("hostResult"), hostResult);
  return withHostStatusProps(out, hostResult);
}

QVariantMap MosaicHost::withHostStatusProps(
    QVariantMap response,
    const QVariantMap &hostResult) const {
  const QVariantMap statusProps = hostStatusProps(hostResult);
  if (statusProps.isEmpty()) {
    return response;
  }
  QVariantMap props = response.value(QStringLiteral("props")).toMap();
  for (auto it = statusProps.cbegin(); it != statusProps.cend(); ++it) {
    props.insert(it.key(), it.value());
  }
  response.insert(QStringLiteral("props"), props);
  return response;
}

QVariantMap MosaicHost::hostStatusProps(const QVariantMap &hostResult) const {
  const QString status = hostResult.value(QStringLiteral("status")).toString();
  if (status.isEmpty()) {
    return {};
  }
  return {
      {QStringLiteral("hostStatusVisible"), true},
      {QStringLiteral("hostStatusKind"), status},
      {QStringLiteral("hostStatusLabel"), hostStatusLabel(status)},
      {QStringLiteral("hostStatusMessage"), hostStatusMessage(hostResult, status)}};
}

QString MosaicHost::hostStatusLabel(const QString &status) const {
  if (status == QStringLiteral("imported")) {
    return QStringLiteral("Import complete");
  }
  if (status == QStringLiteral("exported")) {
    return QStringLiteral("Export complete");
  }
  if (status == QStringLiteral("cancelled")) {
    return QStringLiteral("Import cancelled");
  }
  if (status == QStringLiteral("read-error") || status == QStringLiteral("import-error")) {
    return QStringLiteral("Import failed");
  }
  if (status == QStringLiteral("export-error") || status == QStringLiteral("write-error")) {
    return QStringLiteral("Export failed");
  }
  return QStringLiteral("Host status");
}

QString MosaicHost::hostStatusMessage(
    const QVariantMap &hostResult,
    const QString &status) const {
  const QString file = hostResultFile(hostResult);
  const QString error = hostResult.value(QStringLiteral("error")).toString();
  if (status == QStringLiteral("imported")) {
    return file.isEmpty() ? QStringLiteral("Anki package imported.")
                          : QStringLiteral("Imported %1.").arg(file);
  }
  if (status == QStringLiteral("exported")) {
    return file.isEmpty() ? QStringLiteral("Anki package exported.")
                          : QStringLiteral("Saved %1.").arg(file);
  }
  if (status == QStringLiteral("cancelled")) {
    return QStringLiteral("No Anki package was selected.");
  }
  if (status == QStringLiteral("read-error")) {
    const QString subject = file.isEmpty() ? QStringLiteral("the selected file") : file;
    return error.isEmpty() ? QStringLiteral("Could not read %1.").arg(subject)
                           : QStringLiteral("Could not read %1: %2").arg(subject, error);
  }
  if (status == QStringLiteral("import-error")) {
    const QString subject = file.isEmpty() ? QStringLiteral("the selected package") : file;
    return error.isEmpty() ? QStringLiteral("Could not import %1.").arg(subject)
                           : QStringLiteral("Could not import %1: %2").arg(subject, error);
  }
  if (status == QStringLiteral("export-error")) {
    return error.isEmpty() ? QStringLiteral("Could not export Anki package.")
                           : QStringLiteral("Could not export Anki package: %1").arg(error);
  }
  if (status == QStringLiteral("write-error")) {
    const QString subject = file.isEmpty() ? QStringLiteral("the Anki package") : file;
    return error.isEmpty() ? QStringLiteral("Could not save %1.").arg(subject)
                           : QStringLiteral("Could not save %1: %2").arg(subject, error);
  }
  if (!error.isEmpty()) {
    return error;
  }
  return file.isEmpty() ? status : file;
}

QString MosaicHost::hostResultFile(const QVariantMap &hostResult) const {
  const QString path = hostResult.value(QStringLiteral("path")).toString();
  return path.isEmpty() ? QString() : QFileInfo(path).fileName();
}

QVariantMap MosaicHost::camelCaseProps(const QVariantMap &props) const {
  QVariantMap out;
  for (auto it = props.cbegin(); it != props.cend(); ++it) {
    out.insert(mosaicPropName(it.key()), it.value());
  }
  return out;
}

QString MosaicHost::mosaicPropName(const QString &name) const {
  QString out;
  bool uppercaseNext = false;
  for (const QChar ch : name) {
    if (ch == QLatin1Char('-')) {
      uppercaseNext = true;
      continue;
    }
    if (out.isEmpty()) {
      out.append(ch.toLower());
    } else if (uppercaseNext) {
      out.append(ch.toUpper());
      uppercaseNext = false;
    } else {
      out.append(ch);
    }
  }
  return out;
}

QStringList MosaicHost::hostIntentExtensions(
    const QVariantMap &hostIntent,
    const QString &property,
    const QStringList &fallback) const {
  const QVariantList raw = hostIntent.value(property).toList();
  QStringList out;
  for (const QVariant &value : raw) {
    QString extension = value.toString().trimmed();
    if (extension.isEmpty()) {
      continue;
    }
    if (!extension.startsWith(QLatin1Char('.'))) {
      extension.prepend(QLatin1Char('.'));
    }
    out.push_back(extension);
  }
  return out.isEmpty() ? fallback : out;
}

QString MosaicHost::ankiFileFilter(const QStringList &extensions) const {
  QStringList patterns;
  for (const QString &extension : extensions) {
    patterns.push_back(QStringLiteral("*") + extension);
  }
  return QStringLiteral("Anki packages (%1)").arg(patterns.join(QLatin1Char(' ')));
}

QString MosaicHost::suggestedAnkiFileName(const QVariantMap &hostIntent) const {
  QString name = hostIntent.value(QStringLiteral("deckId")).toString();
  if (name.trimmed().isEmpty()) {
    name = QStringLiteral("engram-collection");
  }
  const QString invalidFileNameChars = QStringLiteral("/\\:*?\"<>|");
  for (const QChar ch : invalidFileNameChars) {
    name.replace(ch, QLatin1Char('-'));
  }
  if (!name.endsWith(QStringLiteral(".apkg"), Qt::CaseInsensitive)) {
    name += QStringLiteral(".apkg");
  }
  return name;
}

QByteArray MosaicHost::jsonByteArray(const QJsonObject &root, const QString &property) const {
  const QJsonArray array = root.value(property).toArray();
  QByteArray out;
  out.reserve(array.size());
  for (const QJsonValue &value : array) {
    out.append(static_cast<char>(value.toInt()));
  }
  return out;
}

QString MosaicHost::snapshotPath() const {
  const QByteArray configured = qgetenv("ENGRAM_SNAPSHOT_PATH");
  if (!configured.isEmpty()) {
    return QString::fromLocal8Bit(configured);
  }
  return QDir::home().filePath(QStringLiteral(".engram/mosaic-snapshot.v1.json"));
}

QByteArray MosaicHost::deckId() const {
  return qgetenv("ENGRAM_DECK_ID");
}

quint64 MosaicHost::nowMs() const {
  return static_cast<quint64>(QDateTime::currentMSecsSinceEpoch());
}
