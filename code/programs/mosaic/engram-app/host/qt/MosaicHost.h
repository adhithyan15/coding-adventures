#ifndef ENGRAM_MOSAIC_HOST_H
#define ENGRAM_MOSAIC_HOST_H

#include <cstddef>

#include <QByteArray>
#include <QJsonObject>
#include <QLibrary>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>
#include <QtGlobal>

class MosaicHost final : public QObject {
  Q_OBJECT

public:
  explicit MosaicHost(QObject *parent = nullptr);
  ~MosaicHost() override;

  Q_INVOKABLE QVariantMap props();
  Q_INVOKABLE QVariantMap handleEvent(const QVariantMap &event);

private:
  using EgSessionNewDemoFn = void *(*)();
  using EgSessionFreeFn = void (*)(void *);
  using EgStringFreeFn = void (*)(char *);
  using EgSnapshotFn = char *(*)(void *);
  using EgLoadSnapshotFn = char *(*)(void *, const char *);
  using EgEngramAppPropsFn = char *(*)(void *, const char *, quint64);
  using EgHandleEngramAppEventFn = char *(*)(void *, const char *, const char *, quint64);
  using EgExportAnkiApkgFn = char *(*)(void *);
  using EgMergeAnkiApkgFn = char *(*)(void *, const quint8 *, std::size_t);

  bool ensureLoaded();
  void hydrateSession();
  void persistSnapshot();
  QString takeCString(char *value) const;
  QVariantMap hostResponseFromJson(const QString &json) const;
  QVariantMap handleHostIntent(const QVariantMap &response);
  QVariantMap importAnkiPackage(const QVariantMap &response, const QVariantMap &hostIntent);
  QVariantMap exportAnkiPackage(const QVariantMap &response, const QVariantMap &hostIntent);
  QVariantMap hostResultResponse(
      const QVariantMap &response,
      const QVariantMap &hostIntent,
      const QString &status,
      const QString &path = QString(),
      const QString &error = QString()) const;
  QVariantMap withHostStatusProps(QVariantMap response, const QVariantMap &hostResult) const;
  QVariantMap hostStatusProps(const QVariantMap &hostResult) const;
  QString hostStatusLabel(const QString &status) const;
  QString hostStatusMessage(const QVariantMap &hostResult, const QString &status) const;
  QString hostResultFile(const QVariantMap &hostResult) const;
  QVariantMap camelCaseProps(const QVariantMap &props) const;
  QString mosaicPropName(const QString &name) const;
  QStringList hostIntentExtensions(
      const QVariantMap &hostIntent,
      const QString &property,
      const QStringList &fallback) const;
  QString ankiFileFilter(const QStringList &extensions) const;
  QString suggestedAnkiFileName(const QVariantMap &hostIntent) const;
  QByteArray jsonByteArray(const QJsonObject &root, const QString &property) const;
  QString snapshotPath() const;
  QByteArray deckId() const;
  quint64 nowMs() const;

  QLibrary library_;
  void *session_ = nullptr;
  EgSessionNewDemoFn sessionNewDemo_ = nullptr;
  EgSessionFreeFn sessionFree_ = nullptr;
  EgStringFreeFn stringFree_ = nullptr;
  EgSnapshotFn snapshot_ = nullptr;
  EgLoadSnapshotFn loadSnapshot_ = nullptr;
  EgEngramAppPropsFn engramAppProps_ = nullptr;
  EgHandleEngramAppEventFn handleEngramAppEvent_ = nullptr;
  EgExportAnkiApkgFn exportAnkiApkg_ = nullptr;
  EgMergeAnkiApkgFn mergeAnkiApkg_ = nullptr;
};

#endif
