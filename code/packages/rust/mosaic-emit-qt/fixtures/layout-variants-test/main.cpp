// UI48 §7.10 (ENV2/ENV3 on Qt): the gate §7 asks for -- it CHANGES the
// environment and asserts the root swaps. A test that only rendered at one
// size would pass against a shell frozen on one layout.
//
// How it runs the real shell
// --------------------------
// CI generates the fixture's sample project
// (`mosaic-compile pkg fixtures/layout-variants --backend qt --emit-project`),
// renames the generated `main.cpp` to `generated_main.cpp` and puts this file
// in its place. This file then compiles the generated one VERBATIM, with its
// `main` renamed, and runs it:
//
//   our main ──► mosaicGeneratedMain   (the generated shell: view, selector,
//                     │                  host, observer, app.exec())
//                     └── QCoreApplication constructed
//                           └── Q_COREAPP_STARTUP_FUNCTION queues the gate,
//                               which runs inside the shell's event loop
//
// So everything asserted below -- the startup choice, the switch on resize,
// the props carried across -- is the generated code doing it, not a copy.
//
// The fixture has a default layout (`LayoutProbe`, a RowLayout) and a compact
// one (`LayoutProbeCompact`, a ColumnLayout, in LayoutProbe.compact.qml),
// selected by the conventional rule `compact` <- `sizeClass == compact`: a
// window narrower than 600 logical pixels. The sample shell has a host but no
// runtime (no application library is installed), so the host reports to
// nobody and the props are what the shell's root shows -- which the gate
// sets, and expects to find on the other root after each swap.
//
// Exit status: 0 when every check passed, 1 on the first failure (printed).

#include <QCoreApplication>
#include <QElapsedTimer>
#include <QGuiApplication>
#include <QQuickItem>
#include <QQuickView>
#include <QString>
#include <QTimer>
#include <QVariantMap>
#include <QWindow>

#include <cstdlib>
#include <iostream>

#define main mosaicGeneratedMain
#include "generated_main.cpp"
#undef main

namespace {

const QString CarriedTitle = QStringLiteral("Carried across layouts");

// Stop the shell's event loop with a failure, saying what was expected.
bool fail(const QString &what)
{
    std::cerr << "Qt layout-variant gate FAILED: " << what.toStdString() << std::endl;
    QCoreApplication::exit(EXIT_FAILURE);
    return false;
}

QQuickView *shellView()
{
    for (auto *window : QGuiApplication::topLevelWindows()) {
        if (auto *view = qobject_cast<QQuickView *>(window)) return view;
    }
    return nullptr;
}

// Let the window system deliver the resize, and the observer react, for at
// most two seconds; true once `done` holds.
template <typename Done>
bool settle(Done done)
{
    QElapsedTimer clock;
    clock.start();
    while (!done() && clock.elapsed() < 2000) {
        QCoreApplication::processEvents(QEventLoop::AllEvents, 50);
    }
    return done();
}

// Which layout is mounted, by its root's file and by what it laid out: the
// default is a RowLayout, the compact one a ColumnLayout.
bool expectLayout(QQuickView &view, const char *file, const char *layoutClass, int width)
{
    const QString at = QStringLiteral(" at %1 x 800").arg(width);
    if (!settle([&] { return view.source().toString().endsWith(QLatin1String(file)); })) {
        return fail(QStringLiteral("expected %1%2, showing %3")
                        .arg(QLatin1String(file), at, view.source().toString()));
    }
    auto *root = view.rootObject();
    if (view.status() != QQuickView::Ready || root == nullptr) {
        return fail(QStringLiteral("%1 did not load%2").arg(QLatin1String(file), at));
    }
    bool laidOut = false;
    for (auto *item : root->findChildren<QQuickItem *>()) {
        if (QString::fromLatin1(item->metaObject()->className()).contains(QLatin1String(layoutClass))) {
            laidOut = true;
        }
    }
    if (!laidOut) return fail(QStringLiteral("%1 has no %2%3").arg(QLatin1String(file), QLatin1String(layoutClass), at));
    // The same props reach the other root.
    if (root->property("title").toString() != CarriedTitle) {
        return fail(QStringLiteral("%1 shows title \"%2\"%3, not the one the previous root showed")
                        .arg(QLatin1String(file), root->property("title").toString(), at));
    }
    // And the host is attached to it, so its events still reach the runtime.
    if (root->property("mosaicHost").value<QObject *>() == nullptr) {
        return fail(QStringLiteral("%1 has no mosaicHost%2").arg(QLatin1String(file), at));
    }
    return true;
}

bool resizeTo(QQuickView &view, int width, const char *file, const char *layoutClass)
{
    view.resize(width, 800);
    return expectLayout(view, file, layoutClass, width);
}

bool selectorChecks()
{
    // The generated rules are the convention, under the wire name.
    const auto &rules = mosaicLayoutRules();
    if (rules.size() != 1 || QLatin1String(rules[0].variant) != QLatin1String("compact")
        || rules[0].conditions.size() != 1
        || QLatin1String(rules[0].conditions[0].first) != QLatin1String("sizeClass")
        || QLatin1String(rules[0].conditions[0].second) != QLatin1String("compact")) {
        return fail(QStringLiteral("the generated rules are not [compact <- sizeClass == compact]"));
    }
    // First match, else the default; a report without the axis never matches.
    if (mosaicLayoutVariant({{QStringLiteral("sizeClass"), QStringLiteral("compact")},
                             {QStringLiteral("pointer"), QStringLiteral("fine")}})
            != QLatin1String("compact")
        || !mosaicLayoutVariant({{QStringLiteral("sizeClass"), QStringLiteral("regular")}}).isEmpty()
        || !mosaicLayoutVariant({{QStringLiteral("sizeClass"), QStringLiteral("expanded")}}).isEmpty()
        || !mosaicLayoutVariant({}).isEmpty()) {
        return fail(QStringLiteral("mosaicLayoutVariant is not select_variant"));
    }
    return true;
}

void runGate()
{
    auto *view = shellView();
    if (view == nullptr) {
        fail(QStringLiteral("the generated shell showed no QQuickView"));
        return;
    }
    if (!selectorChecks()) return;

    // The shell opens at 1100 x 800: regular, so the default layout. Give its
    // root a title the runtime would have; every later root must show it.
    if (!settle([&] { return view->rootObject() != nullptr; })) {
        fail(QStringLiteral("the shell mounted no root"));
        return;
    }
    view->rootObject()->setProperty("title", CarriedTitle);
    if (!expectLayout(*view, "/LayoutProbe.qml", "RowLayout", 1100)) return;

    // Narrower than 600: compact. Nothing but the window changed, so nothing
    // else could have swapped the root.
    if (!resizeTo(*view, 400, "/LayoutProbe.compact.qml", "ColumnLayout")) return;
    // Just under and at the threshold: the bucket, not the pixel, decides.
    if (!resizeTo(*view, 599, "/LayoutProbe.compact.qml", "ColumnLayout")) return;
    if (!resizeTo(*view, 600, "/LayoutProbe.qml", "RowLayout")) return;
    // And back again, and wider still.
    if (!resizeTo(*view, 400, "/LayoutProbe.compact.qml", "ColumnLayout")) return;
    if (!resizeTo(*view, 1200, "/LayoutProbe.qml", "RowLayout")) return;

    std::cout << "Qt layout-variant resize gate passed" << std::endl;
    QCoreApplication::exit(EXIT_SUCCESS);
}

// Runs as the generated shell constructs its QApplication; the gate itself
// waits for the shell's event loop, by which time the view is showing.
void queueGate()
{
    QTimer::singleShot(0, [] { runGate(); });
}

} // namespace

Q_COREAPP_STARTUP_FUNCTION(queueGate)

int main(int argc, char *argv[])
{
    return mosaicGeneratedMain(argc, argv);
}
