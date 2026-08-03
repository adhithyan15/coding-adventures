#include "MosaicHost.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QHoverEvent>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QTimer>
#include <QUrl>
#include <QWheelEvent>
#include <QtQml/qqml.h>

#include <cmath>

MosaicHost *MosaicHost::active_ = nullptr;

namespace {

QString bridgeFileName()
{
#if defined(Q_OS_WIN)
  return QStringLiteral("venture_browser_qt.dll");
#elif defined(Q_OS_MACOS)
  return QStringLiteral("libventure_browser_qt.dylib");
#else
  return QStringLiteral("libventure_browser_qt.so");
#endif
}

QByteArray keyCommand(QKeyEvent *event)
{
  switch (event->key()) {
  case Qt::Key_Up: return "line-up";
  case Qt::Key_Down: return "line-down";
  case Qt::Key_PageUp: return "page-up";
  case Qt::Key_PageDown: return "page-down";
  case Qt::Key_Home: return "start";
  case Qt::Key_End: return "end";
  case Qt::Key_Space:
    return event->modifiers().testFlag(Qt::ShiftModifier) ? "page-up" : "page-down";
  default: return {};
  }
}

} // namespace

VentureContentSurface::VentureContentSurface(QQuickItem *parent)
  : QQuickPaintedItem(parent), host_(MosaicHost::active())
{
  setAcceptedMouseButtons(Qt::LeftButton);
  setAcceptHoverEvents(true);
  setActiveFocusOnTab(true);
  setAntialiasing(true);
}

void VentureContentSurface::paint(QPainter *painter)
{
  QImage image;
  if (host_ && host_->render(&image)) {
    painter->drawImage(boundingRect(), image);
  }
}

void VentureContentSurface::geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry)
{
  QQuickPaintedItem::geometryChange(newGeometry, oldGeometry);
  if (host_ && host_->resize(newGeometry.width(), newGeometry.height())) {
    update();
  }
}

void VentureContentSurface::hoverMoveEvent(QHoverEvent *event)
{
  if (host_ && host_->updateHover(event->position().x(), event->position().y())) {
    host_->publishProps();
  }
  QQuickPaintedItem::hoverMoveEvent(event);
}

void VentureContentSurface::hoverLeaveEvent(QHoverEvent *event)
{
  if (host_ && host_->updateHover(NAN, NAN)) {
    host_->publishProps();
  }
  QQuickPaintedItem::hoverLeaveEvent(event);
}

void VentureContentSurface::mouseReleaseEvent(QMouseEvent *event)
{
  if (host_ && event->button() == Qt::LeftButton
      && host_->activateLink(event->position().x(), event->position().y())) {
    host_->publishProps();
    update();
  }
  QQuickPaintedItem::mouseReleaseEvent(event);
}

void VentureContentSurface::wheelEvent(QWheelEvent *event)
{
  if (host_ && host_->scroll(-event->angleDelta().y())) {
    update();
    event->accept();
    return;
  }
  QQuickPaintedItem::wheelEvent(event);
}

void VentureContentSurface::keyPressEvent(QKeyEvent *event)
{
  const QByteArray command = keyCommand(event);
  if (host_ && !command.isEmpty() && host_->scrollCommand(command)) {
    update();
    event->accept();
    return;
  }
  QQuickPaintedItem::keyPressEvent(event);
}

MosaicHost::MosaicHost(QObject *parent) : QObject(parent)
{
  active_ = this;
  loadBridge();
}

MosaicHost::~MosaicHost()
{
  if (browser_ && free_) {
    free_(browser_);
  }
  if (active_ == this) {
    active_ = nullptr;
  }
}

void MosaicHost::registerTypes()
{
  qmlRegisterType<VentureContentSurface>("Mosaic.VentureHost", 1, 0, "VentureContentSurface");
}

MosaicHost *MosaicHost::active() { return active_; }

void MosaicHost::attach(QObject *root)
{
  root_ = root;
  QQmlEngine *engine = qmlEngine(root);
  if (!engine || contentComponent_) {
    return;
  }
  contentComponent_ = new QQmlComponent(engine, this);
  contentComponent_->setData(
    "import QtQuick 2.15\nimport Mosaic.VentureHost 1.0\nVentureContentSurface {}\n",
    QUrl(QStringLiteral("mosaic-host:VentureContentSurface.qml")));
  scheduleAcceptance();
}

QVariantMap MosaicHost::props()
{
  if (!browser_ || !applyProps_) {
    return {{QStringLiteral("error"), QStringLiteral("Venture Qt bridge unavailable")}};
  }
  return withContentSurface(response(applyProps_(browser_)));
}

QVariantMap MosaicHost::handleEvent(const QVariantMap &event)
{
  if (!browser_ || !handleEvent_) {
    return {{QStringLiteral("error"), QStringLiteral("Venture Qt bridge unavailable")}};
  }
  const QByteArray name = event.value(QStringLiteral("event")).toString().toUtf8();
  const QByteArray value = event.value(QStringLiteral("value")).toString().toUtf8();
  return withContentSurface(response(handleEvent_(browser_, name.constData(), value.constData())));
}

bool MosaicHost::render(QImage *image)
{
  if (!browser_ || !render_ || !image) {
    return false;
  }
  unsigned int width = 0;
  unsigned int height = 0;
  const size_t required = render_(browser_, nullptr, 0, &width, &height);
  if (!required || !width || !height) {
    return false;
  }
  QByteArray pixels(static_cast<qsizetype>(required), Qt::Uninitialized);
  if (render_(browser_, reinterpret_cast<unsigned char *>(pixels.data()), required,
              &width, &height) != required) {
    return false;
  }
  QImage borrowed(reinterpret_cast<const unsigned char *>(pixels.constData()),
                  static_cast<int>(width), static_cast<int>(height),
                  QImage::Format_RGBA8888);
  *image = borrowed.copy();
  return !image->isNull();
}

bool MosaicHost::resize(double width, double height)
{
  return browser_ && resize_ && width > 0 && height > 0
    && resize_(browser_, width, height) != 0;
}

bool MosaicHost::scroll(double deltaY)
{
  return browser_ && scroll_ && scroll_(browser_, deltaY) != 0;
}

bool MosaicHost::scrollCommand(const QByteArray &command)
{
  return browser_ && scrollCommand_ && scrollCommand_(browser_, command.constData()) != 0;
}

bool MosaicHost::activateLink(double x, double y)
{
  return browser_ && activateLink_ && activateLink_(browser_, x, y) != 0;
}

bool MosaicHost::updateHover(double x, double y)
{
  return browser_ && updateHover_ && updateHover_(browser_, x, y) != 0;
}

void MosaicHost::publishProps()
{
  if (root_) {
    QMetaObject::invokeMethod(root_, "applyMosaicResponse",
                              Q_ARG(QVariant, QVariant::fromValue(props())));
  }
}

bool MosaicHost::loadBridge()
{
  const QString configured = QString::fromUtf8(qgetenv("VENTURE_BROWSER_LIBRARY"));
  const QString path = configured.isEmpty()
    ? QDir(QCoreApplication::applicationDirPath()).filePath(bridgeFileName())
    : configured;
  library_.setFileName(path);
  if (!library_.load()) {
    return false;
  }

#define RESOLVE(member, symbol) \
  member = reinterpret_cast<decltype(member)>(library_.resolve("venture_browser_qt_" symbol))
  RESOLVE(new_, "new");
  RESOLVE(free_, "free");
  RESOLVE(applyProps_, "apply_props");
  RESOLVE(handleEvent_, "handle_event");
  RESOLVE(scroll_, "scroll");
  RESOLVE(scrollCommand_, "scroll_command");
  RESOLVE(activateLink_, "activate_link");
  RESOLVE(updateHover_, "update_hover");
  RESOLVE(resize_, "resize");
  RESOLVE(render_, "render_rgba");
  RESOLVE(stringFree_, "string_free");
#undef RESOLVE

  if (!new_ || !free_ || !applyProps_ || !handleEvent_ || !scroll_
      || !scrollCommand_ || !activateLink_ || !updateHover_ || !resize_
      || !render_ || !stringFree_) {
    library_.unload();
    return false;
  }
  QByteArray startUrl = qgetenv("VENTURE_START_URL");
  if (startUrl.isEmpty()) {
    startUrl = "http://info.cern.ch/";
  }
  browser_ = new_(startUrl.constData(), 1024.0, 640.0);
  return browser_ != nullptr;
}

void MosaicHost::scheduleAcceptance()
{
  const QByteArray markerPath = qgetenv("VENTURE_BROWSER_ACCEPTANCE_PATH");
  if (markerPath.isEmpty()) {
    return;
  }

  QTimer::singleShot(250, this, [this, markerPath]() {
    const QVariantMap browserResponse = props();
    const QVariantMap browserProps = browserResponse.value(QStringLiteral("props")).toMap();
    QImage frame;
    const bool rendered = render(&frame);
    const bool componentReady = contentComponent_ && !contentComponent_->isError();
    const bool surfaceMounted = root_ && root_->findChild<VentureContentSurface *>() != nullptr;
    const bool ok = browser_ && rendered && componentReady && surfaceMounted
      && !browserProps.value(QStringLiteral("address")).toString().isEmpty()
      && !browserProps.value(QStringLiteral("pageTitle")).toString().isEmpty();

    QJsonObject report {
      {QStringLiteral("ok"), ok},
      {QStringLiteral("address"), browserProps.value(QStringLiteral("address")).toString()},
      {QStringLiteral("pageTitle"), browserProps.value(QStringLiteral("pageTitle")).toString()},
      {QStringLiteral("rendered"), rendered},
      {QStringLiteral("componentReady"), componentReady},
      {QStringLiteral("surfaceMounted"), surfaceMounted},
      {QStringLiteral("width"), frame.width()},
      {QStringLiteral("height"), frame.height()},
    };
    QFile marker(QString::fromUtf8(markerPath));
    if (marker.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
      marker.write(QJsonDocument(report).toJson(QJsonDocument::Compact));
      marker.close();
    }
    QCoreApplication::exit(ok ? 0 : 2);
  });
}

QVariantMap MosaicHost::response(char *json) const
{
  if (!json) {
    return {};
  }
  const QByteArray bytes(json);
  stringFree_(json);
  const QJsonDocument document = QJsonDocument::fromJson(bytes);
  return document.isObject() ? document.object().toVariantMap() : QVariantMap{};
}

QVariantMap MosaicHost::withContentSurface(QVariantMap response) const
{
  QVariantMap next = response.value(QStringLiteral("props")).toMap();
  next = normalizeProps(next);
  if (contentComponent_) {
    next.insert(QStringLiteral("contentSurface"),
                QVariant::fromValue(static_cast<QObject *>(contentComponent_)));
  }
  response.insert(QStringLiteral("props"), next);
  return response;
}

QVariantMap MosaicHost::normalizeProps(const QVariantMap &props)
{
  static const QHash<QString, QString> names = {
    {QStringLiteral("page-title"), QStringLiteral("pageTitle")},
    {QStringLiteral("status-text"), QStringLiteral("statusText")},
    {QStringLiteral("back-disabled"), QStringLiteral("backDisabled")},
    {QStringLiteral("forward-disabled"), QStringLiteral("forwardDisabled")},
    {QStringLiteral("navigation-disabled"), QStringLiteral("navigationDisabled")},
    {QStringLiteral("content-surface"), QStringLiteral("contentSurface")},
  };
  QVariantMap normalized;
  for (auto it = props.cbegin(); it != props.cend(); ++it) {
    normalized.insert(names.value(it.key(), it.key()), it.value());
  }
  return normalized;
}
