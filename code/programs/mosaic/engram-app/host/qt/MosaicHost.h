#ifndef ENGRAM_MOSAIC_HOST_H
#define ENGRAM_MOSAIC_HOST_H

#include <QByteArray>
#include <QLibrary>
#include <QObject>
#include <QString>
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
  using EgSessionNewFn = void *(*)();
  using EgSessionFreeFn = void (*)(void *);
  using EgStringFreeFn = void (*)(char *);
  using EgEngramAppPropsFn = char *(*)(void *, const char *, quint64);
  using EgHandleEngramAppEventFn = char *(*)(void *, const char *, const char *, quint64);

  bool ensureLoaded();
  QString takeCString(char *value) const;
  QVariantMap propsFromResponse(const QString &json) const;
  QVariantMap camelCaseProps(const QVariantMap &props) const;
  QString mosaicPropName(const QString &name) const;
  QByteArray deckId() const;
  quint64 nowMs() const;

  QLibrary library_;
  void *session_ = nullptr;
  EgSessionNewFn sessionNew_ = nullptr;
  EgSessionFreeFn sessionFree_ = nullptr;
  EgStringFreeFn stringFree_ = nullptr;
  EgEngramAppPropsFn engramAppProps_ = nullptr;
  EgHandleEngramAppEventFn handleEngramAppEvent_ = nullptr;
};

#endif
