#include "MosaicHost.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDebug>
#include <QDir>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
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
  return hostResponseFromJson(takeCString(
      handleEngramAppEvent_(session_, eventJson.constData(), deck.constData(), nowMs())));
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

  sessionNew_ = resolveSymbol<EgSessionNewFn>(library_, "eg_session_new");
  sessionFree_ = resolveSymbol<EgSessionFreeFn>(library_, "eg_session_free");
  stringFree_ = resolveSymbol<EgStringFreeFn>(library_, "eg_string_free");
  engramAppProps_ = resolveSymbol<EgEngramAppPropsFn>(library_, "eg_engram_app_props");
  handleEngramAppEvent_ =
      resolveSymbol<EgHandleEngramAppEventFn>(library_, "eg_handle_engram_app_event");

  if (sessionNew_ == nullptr || sessionFree_ == nullptr || stringFree_ == nullptr ||
      engramAppProps_ == nullptr || handleEngramAppEvent_ == nullptr) {
    qWarning() << "Engram MosaicHost loaded engram-capi but required symbols are missing";
    library_.unload();
    return false;
  }

  session_ = sessionNew_();
  if (session_ == nullptr) {
    qWarning() << "Engram MosaicHost failed to create an Engram session";
    return false;
  }
  return true;
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

QByteArray MosaicHost::deckId() const {
  return qgetenv("ENGRAM_DECK_ID");
}

quint64 MosaicHost::nowMs() const {
  return static_cast<quint64>(QDateTime::currentMSecsSinceEpoch());
}
