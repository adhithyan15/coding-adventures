#pragma once

#include <QByteArray>
#include <QImage>
#include <QLibrary>
#include <QObject>
#include <QPointer>
#include <QQuickPaintedItem>
#include <QVariantMap>

class QQmlComponent;
class MosaicHost;

class VentureContentSurface : public QQuickPaintedItem
{
  Q_OBJECT
public:
  explicit VentureContentSurface(QQuickItem *parent = nullptr);
  void paint(QPainter *painter) override;

protected:
  void geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry) override;
  void hoverMoveEvent(QHoverEvent *event) override;
  void hoverLeaveEvent(QHoverEvent *event) override;
  void mouseReleaseEvent(QMouseEvent *event) override;
  void wheelEvent(QWheelEvent *event) override;
  void keyPressEvent(QKeyEvent *event) override;

private:
  MosaicHost *host_ = nullptr;
};

class MosaicHost final : public QObject
{
  Q_OBJECT
public:
  explicit MosaicHost(QObject *parent = nullptr);
  ~MosaicHost() override;

  static void registerTypes();
  static MosaicHost *active();

  void attach(QObject *root);
  Q_INVOKABLE QVariantMap props();
  Q_INVOKABLE QVariantMap handleEvent(const QVariantMap &event);

  bool render(QImage *image);
  bool resize(double width, double height);
  bool scroll(double deltaY);
  bool scrollCommand(const QByteArray &command);
  bool activateLink(double x, double y);
  bool updateHover(double x, double y);
  void publishProps();

private:
  using NewFn = void *(*)(const char *, double, double);
  using FreeFn = void (*)(void *);
  using ApplyPropsFn = char *(*)(void *);
  using HandleEventFn = char *(*)(void *, const char *, const char *);
  using ScrollFn = unsigned char (*)(void *, double);
  using ScrollCommandFn = unsigned char (*)(void *, const char *);
  using ScrollMetricsFn = unsigned char (*)(void *, double *, double *, double *, double *);
  using PointFn = unsigned char (*)(void *, double, double);
  using ResizeFn = unsigned char (*)(void *, double, double);
  using RenderFn = size_t (*)(void *, unsigned char *, size_t, unsigned int *, unsigned int *);
  using StringFreeFn = void (*)(char *);

  bool loadBridge();
  void scheduleAcceptance();
  void runInteractionAcceptance(const QByteArray &markerPath,
                                const QByteArray &targetUrl,
                                const QByteArray &linkUrl);
  bool scrollOffset(double *offsetY) const;
  QVariantMap response(char *json) const;
  QVariantMap withContentSurface(QVariantMap response) const;
  static QVariantMap normalizeProps(const QVariantMap &props);

  static MosaicHost *active_;
  QLibrary library_;
  void *browser_ = nullptr;
  QPointer<QObject> root_;
  QQmlComponent *contentComponent_ = nullptr;
  NewFn new_ = nullptr;
  FreeFn free_ = nullptr;
  ApplyPropsFn applyProps_ = nullptr;
  HandleEventFn handleEvent_ = nullptr;
  ScrollFn scroll_ = nullptr;
  ScrollCommandFn scrollCommand_ = nullptr;
  ScrollMetricsFn scrollMetrics_ = nullptr;
  PointFn activateLink_ = nullptr;
  PointFn updateHover_ = nullptr;
  ResizeFn resize_ = nullptr;
  RenderFn render_ = nullptr;
  StringFreeFn stringFree_ = nullptr;
};
