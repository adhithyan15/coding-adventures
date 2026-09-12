#include "MosaicHost.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QHoverEvent>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QMimeDatabase>
#include <QPainter>
#include <QQuickItem>
#include <QQuickWindow>
#include <QQmlComponent>
#include <QQmlEngine>
#include <QTimer>
#include <QUuid>
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

QByteArray controlKey(QKeyEvent *event)
{
  const auto modifiers = event->modifiers();
  const bool command = modifiers.testFlag(Qt::ControlModifier)
    || modifiers.testFlag(Qt::MetaModifier);
  if (command) {
    if (event->key() == Qt::Key_A) return "select-all";
    if (event->key() == Qt::Key_Z) {
      return modifiers.testFlag(Qt::ShiftModifier) ? "redo" : "undo";
    }
    if (event->key() == Qt::Key_Y) return "redo";
  }
  if (modifiers.testFlag(Qt::AltModifier)) {
    if (event->key() == Qt::Key_Left) return "word-left";
    if (event->key() == Qt::Key_Right) return "word-right";
  }
  switch (event->key()) {
  case Qt::Key_Backspace: return "backspace";
  case Qt::Key_Delete: return "delete";
  case Qt::Key_Left: return "arrow-left";
  case Qt::Key_Right: return "arrow-right";
  case Qt::Key_Up: return "arrow-up";
  case Qt::Key_Down: return "arrow-down";
  case Qt::Key_Home: return "home";
  case Qt::Key_End: return "end";
  case Qt::Key_Return:
  case Qt::Key_Enter: return "enter";
  case Qt::Key_Space: return "space";
  default: return {};
  }
}

bool sendKey(QObject *control, int key)
{
  auto *item = qobject_cast<QQuickItem *>(control);
  if (!item) {
    return false;
  }
  if (item->window()) {
    item->window()->requestActivate();
  }
  item->forceActiveFocus();
  QKeyEvent press(QEvent::KeyPress, key, Qt::NoModifier);
  QKeyEvent release(QEvent::KeyRelease, key, Qt::NoModifier);
  QObject *receiver = item->window() ? static_cast<QObject *>(item->window())
                                     : static_cast<QObject *>(item);
  QCoreApplication::sendEvent(receiver, &press);
  QCoreApplication::sendEvent(receiver, &release);
  QCoreApplication::processEvents();
  return true;
}

void finishAcceptance(const QByteArray &markerPath, QJsonObject report, bool ok)
{
  report.insert(QStringLiteral("ok"), ok);
  QFile marker(QString::fromUtf8(markerPath));
  if (marker.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
    marker.write(QJsonDocument(report).toJson(QJsonDocument::Compact));
    marker.close();
  }
  QCoreApplication::exit(ok ? 0 : 2);
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
    host_->presentFilePicker();
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
  if (host_ && event->key() == Qt::Key_U
      && event->modifiers() == Qt::ControlModifier
      && host_->requestViewSource()) {
    event->accept();
    return;
  }
  const QByteArray control = controlKey(event);
  if (host_ && !control.isEmpty()
      && host_->controlKey(control, event->modifiers().testFlag(Qt::ShiftModifier))) {
    update();
    event->accept();
    return;
  }
  if (host_ && !event->text().isEmpty()
      && event->modifiers().testFlag(Qt::ControlModifier) == false
      && event->modifiers().testFlag(Qt::AltModifier) == false
      && event->modifiers().testFlag(Qt::MetaModifier) == false
      && host_->controlText(event->text().toUtf8())) {
    update();
    event->accept();
    return;
  }
  const QByteArray command = keyCommand(event);
  if (host_ && !command.isEmpty() && host_->scrollCommand(command)) {
    update();
    event->accept();
    return;
  }
  QQuickPaintedItem::keyPressEvent(event);
}

bool MosaicHost::requestViewSource()
{
  QVariantMap event;
  event.insert(QStringLiteral("event"), QStringLiteral("onViewSource"));
  return handleEvent(event).value(QStringLiteral("error")).isNull();
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
  auto result = response(handleEvent_(browser_, name.constData(), value.constData()));
  consumeEffect(result);
  return withContentSurface(result);
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

bool MosaicHost::controlKey(const QByteArray &key, bool shift)
{
  return browser_ && controlKey_
    && controlKey_(browser_, key.constData(), shift ? 1 : 0) != 0;
}

bool MosaicHost::controlText(const QByteArray &text)
{
  return browser_ && controlText_ && controlText_(browser_, text.constData()) != 0;
}

bool MosaicHost::presentFilePicker()
{
  if (!browser_ || !filePickerRequest_ || !controlFile_) {
    return false;
  }
  const QVariantMap request = response(filePickerRequest_(browser_));
  if (request.isEmpty()) {
    return false;
  }
  const QByteArray key = request.value("key").toString().toUtf8();
  if (key.isEmpty()) {
    return false;
  }
  const QStringList paths = request.value("multiple").toBool()
    ? QFileDialog::getOpenFileNames(nullptr, tr("Choose files"))
    : QStringList{QFileDialog::getOpenFileName(nullptr, tr("Choose file"))};
  bool changed = false;
  QMimeDatabase mimeDatabase;
  int accepted = 0;
  for (const QString &path : paths) {
    if (path.isEmpty()) {
      continue;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
      continue;
    }
    const QByteArray bytes = file.read(16 * 1024 * 1024 + 1);
    const QByteArray opaque = QUuid::createUuid().toString(QUuid::WithoutBraces).toUtf8();
    const QByteArray name = QFileInfo(path).fileName().toUtf8();
    const QByteArray mediaType = mimeDatabase.mimeTypeForFile(path).name().toUtf8();
    changed = controlFile_(
      browser_, key.constData(), opaque.constData(), name.constData(), mediaType.constData(),
      reinterpret_cast<const unsigned char *>(bytes.constData()),
      static_cast<size_t>(bytes.size()), accepted == 0 ? 0 : 1) != 0 || changed;
    ++accepted;
  }
  if (changed) {
    publishProps();
  }
  return changed;
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
  RESOLVE(controlKey_, "control_key");
  RESOLVE(controlText_, "control_text");
  RESOLVE(controlCopy_, "control_copy");
  RESOLVE(controlCut_, "control_cut");
  RESOLVE(controlPaste_, "control_paste");
  RESOLVE(caretTick_, "caret_tick");
  RESOLVE(imeCandidateRect_, "ime_candidate_rect");
  RESOLVE(filePickerRequest_, "file_picker_request");
  RESOLVE(controlFile_, "control_file");
  RESOLVE(scrollMetrics_, "scroll_metrics");
  RESOLVE(activateLink_, "activate_link");
  RESOLVE(updateHover_, "update_hover");
  RESOLVE(resize_, "resize");
  RESOLVE(render_, "render_rgba");
  RESOLVE(stringFree_, "string_free");
#undef RESOLVE

  if (!new_ || !free_ || !applyProps_ || !handleEvent_ || !scroll_
      || !scrollCommand_ || !controlKey_ || !controlText_ || !controlCopy_
      || !controlCut_ || !controlPaste_ || !caretTick_ || !imeCandidateRect_
      || !filePickerRequest_ || !controlFile_ || !scrollMetrics_
      || !activateLink_ || !updateHover_
      || !resize_ || !render_ || !stringFree_) {
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
    const QByteArray targetUrl = qgetenv("VENTURE_BROWSER_INTERACTION_URL");
    const QByteArray linkUrl = qgetenv("VENTURE_BROWSER_INTERACTION_LINK_URL");
    if (!targetUrl.isEmpty() && !linkUrl.isEmpty()) {
      runInteractionAcceptance(markerPath, targetUrl, linkUrl);
      return;
    }

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
    finishAcceptance(markerPath, report, ok);
  });
}

bool MosaicHost::scrollOffset(double *offsetY) const
{
  if (!browser_ || !scrollMetrics_ || !offsetY) {
    return false;
  }
  double viewportHeight = 0;
  double contentHeight = 0;
  double maxOffsetY = 0;
  return scrollMetrics_(browser_, offsetY, &viewportHeight, &contentHeight,
                        &maxOffsetY) != 0;
}

void MosaicHost::runInteractionAcceptance(const QByteArray &markerPath,
                                          const QByteArray &targetUrl,
                                          const QByteArray &linkUrl)
{
  auto fail = [&markerPath](const QString &message) {
    finishAcceptance(markerPath,
                     {{QStringLiteral("backend"), QStringLiteral("qt")},
                      {QStringLiteral("status"), QStringLiteral("error")},
                      {QStringLiteral("error"), message}},
                     false);
  };
  if (!root_) {
    fail(QStringLiteral("generated Qt root is unavailable"));
    return;
  }

  auto *address = root_->findChild<QObject *>(QStringLiteral("address-input"));
  auto *back = root_->findChild<QObject *>(QStringLiteral("back-button"));
  auto *forward = root_->findChild<QObject *>(QStringLiteral("forward-button"));
  auto *surface = root_->findChild<VentureContentSurface *>();
  if (!address || !back || !forward || !surface) {
    fail(QStringLiteral("generated address/history controls or live surface are unavailable"));
    return;
  }
  const QString startUrl = QString::fromUtf8(qgetenv("VENTURE_START_URL"));
  if (back->property("enabled").toBool() || forward->property("enabled").toBool()) {
    fail(QStringLiteral("initial generated history controls are not disabled"));
    return;
  }

  address->setProperty("text", QString::fromUtf8(targetUrl));
  QCoreApplication::processEvents();
  if (!sendKey(address, Qt::Key_Return)) {
    fail(QStringLiteral("generated address control rejected native Return"));
    return;
  }
  QVariantMap current = props().value(QStringLiteral("props")).toMap();
  if (current.value(QStringLiteral("address")).toString() != QString::fromUtf8(targetUrl)
      || current.value(QStringLiteral("pageTitle")).toString()
           != QStringLiteral("Venture Qt interaction acceptance")
      || !back->property("enabled").toBool()
      || forward->property("enabled").toBool()) {
    fail(QStringLiteral("native address commit did not update shared history state"));
    return;
  }

  if (!sendKey(back, Qt::Key_Space)) {
    fail(QStringLiteral("generated Back control rejected native Space"));
    return;
  }
  current = props().value(QStringLiteral("props")).toMap();
  if (current.value(QStringLiteral("address")).toString() != startUrl
      || !forward->property("enabled").toBool()) {
    fail(QStringLiteral("generated Back control did not traverse shared history"));
    return;
  }

  if (!sendKey(forward, Qt::Key_Space)) {
    fail(QStringLiteral("generated Forward control rejected native Space"));
    return;
  }
  current = props().value(QStringLiteral("props")).toMap();
  if (current.value(QStringLiteral("address")).toString() != QString::fromUtf8(targetUrl)) {
    fail(QStringLiteral("generated Forward control did not traverse shared history"));
    return;
  }
  if (!sendKey(back, Qt::Key_Space)) {
    fail(QStringLiteral("generated Back control could not restore the start page"));
    return;
  }

  double beforeWheel = 0;
  double afterWheel = 0;
  const QPointF wheelPoint(32, 100);
  QWheelEvent wheel(wheelPoint, wheelPoint, QPoint(), QPoint(0, -120), Qt::NoButton,
                    Qt::NoModifier, Qt::ScrollUpdate, false);
  surface->forceActiveFocus();
  if (!scrollOffset(&beforeWheel)) {
    fail(QStringLiteral("shared viewport metrics are unavailable"));
    return;
  }
  QCoreApplication::sendEvent(surface, &wheel);
  QCoreApplication::processEvents();
  if (!wheel.isAccepted() || !scrollOffset(&afterWheel) || afterWheel <= beforeWheel) {
    fail(QStringLiteral("native wheel did not scroll the shared viewport"));
    return;
  }
  QWheelEvent resetWheel(wheelPoint, wheelPoint, QPoint(), QPoint(0, 120),
                         Qt::NoButton, Qt::NoModifier, Qt::ScrollUpdate, false);
  QCoreApplication::sendEvent(surface, &resetWheel);
  QCoreApplication::processEvents();
  if (!resetWheel.isAccepted() || !scrollOffset(&afterWheel)
      || std::abs(afterWheel) > 0.5) {
    fail(QStringLiteral("native reverse wheel did not reset the shared viewport (offset %1)")
           .arg(afterWheel));
    return;
  }

  const QPointF linkPoint(32, 26);
  QHoverEvent hover(QEvent::HoverMove, linkPoint, linkPoint, QPointF(-1, -1));
  QCoreApplication::sendEvent(surface, &hover);
  QCoreApplication::processEvents();
  current = props().value(QStringLiteral("props")).toMap();
  if (current.value(QStringLiteral("statusText")).toString() != QString::fromUtf8(linkUrl)) {
    fail(QStringLiteral("native hover did not project the live link URL"));
    return;
  }

  QMouseEvent press(QEvent::MouseButtonPress, linkPoint, linkPoint, linkPoint,
                    Qt::LeftButton, Qt::LeftButton, Qt::NoModifier);
  QMouseEvent release(QEvent::MouseButtonRelease, linkPoint, linkPoint, linkPoint,
                      Qt::LeftButton, Qt::NoButton, Qt::NoModifier);
  QCoreApplication::sendEvent(surface, &press);
  QCoreApplication::sendEvent(surface, &release);
  QCoreApplication::processEvents();
  current = props().value(QStringLiteral("props")).toMap();
  QImage frame;
  const bool rendered = render(&frame);
  const bool ok = current.value(QStringLiteral("address")).toString()
                    == QString::fromUtf8(linkUrl)
    && current.value(QStringLiteral("pageTitle")).toString()
         == QStringLiteral("Venture Qt link acceptance")
    && rendered && !frame.isNull();
  finishAcceptance(
    markerPath,
    {{QStringLiteral("backend"), QStringLiteral("qt")},
     {QStringLiteral("status"), ok ? QStringLiteral("interacted")
                                    : QStringLiteral("error")},
     {QStringLiteral("addressCommit"), QStringLiteral("native-return")},
     {QStringLiteral("historyControls"), QStringLiteral("back-forward")},
     {QStringLiteral("surfaceWheel"), QStringLiteral("scroll")},
     {QStringLiteral("surfaceHover"), QStringLiteral("status")},
     {QStringLiteral("surfacePointer"), QStringLiteral("link")},
     {QStringLiteral("address"), current.value(QStringLiteral("address")).toString()},
     {QStringLiteral("pageTitle"), current.value(QStringLiteral("pageTitle")).toString()},
     {QStringLiteral("rendered"), rendered},
     {QStringLiteral("width"), frame.width()},
     {QStringLiteral("height"), frame.height()}},
    ok);
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

void MosaicHost::consumeEffect(const QVariantMap &response)
{
  const QVariantMap effect = response.value(QStringLiteral("effect")).toMap();
  if (effect.value(QStringLiteral("type")).toString()
      == QStringLiteral("open-auxiliary-document")) {
    emit auxiliaryDocumentRequested(effect.value(QStringLiteral("document")).toMap());
  }
}

QVariantMap MosaicHost::normalizeProps(const QVariantMap &props)
{
  static const QHash<QString, QString> names = {
    {QStringLiteral("page-title"), QStringLiteral("pageTitle")},
    {QStringLiteral("status-text"), QStringLiteral("statusText")},
    {QStringLiteral("back-disabled"), QStringLiteral("backDisabled")},
    {QStringLiteral("forward-disabled"), QStringLiteral("forwardDisabled")},
    {QStringLiteral("bookmark-label"), QStringLiteral("bookmarkLabel")},
    {QStringLiteral("bookmark-disabled"), QStringLiteral("bookmarkDisabled")},
    {QStringLiteral("view-source-disabled"), QStringLiteral("viewSourceDisabled")},
    {QStringLiteral("navigation-disabled"), QStringLiteral("navigationDisabled")},
    {QStringLiteral("content-surface"), QStringLiteral("contentSurface")},
  };
  QVariantMap normalized;
  for (auto it = props.cbegin(); it != props.cend(); ++it) {
    normalized.insert(names.value(it.key(), it.key()), it.value());
  }
  return normalized;
}
