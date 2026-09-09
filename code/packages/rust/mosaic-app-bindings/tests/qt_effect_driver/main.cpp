// Drives the emitted Qt host against the conformance runtime.
//
// The Rust tests around this crate assert on the TEXT of the emitted host.
// That cannot tell you it compiles, let alone that it behaves -- and the defect
// this file exists for is behavioural: before the host completed effects, an
// `await` was dropped and the app waited forever with nothing reporting it.
#include "MosaicHost.h"
#include <QCoreApplication>
#include <QVariantMap>
#include <cstdio>

static int failures = 0;

static void check(bool ok, const char *what) {
    std::printf("%-56s %s\n", what, ok ? "ok" : "FAIL");
    if (!ok) failures++;
}

// A missing key converts to 0, so comparing against 0 passes when the props are
// empty. Require the key, or the assertion is vacuous.
static int awaited(const QVariantMap &props, const char *where) {
    if (!props.contains(QStringLiteral("awaitedEffects"))) {
        std::printf("VACUOUS: %s carries no awaitedEffects key\n", where);
        failures++;
        return -1;
    }
    return props.value(QStringLiteral("awaitedEffects")).toInt();
}

static QVariantMap requestAwait(MosaicHost &host) {
    QVariantMap event;
    event.insert(QStringLiteral("name"), QStringLiteral("requestEffect"));
    event.insert(QStringLiteral("payload"),
                 QVariantMap{{QStringLiteral("notify"), false}});
    return host.handleEvent(event).value(QStringLiteral("props")).toMap();
}

int main(int argc, char **argv) {
    QCoreApplication app(argc, argv);

    {
        MosaicHost host;
        const auto startup = host.props();
        if (startup.contains(QStringLiteral("error"))) {
            std::printf("host failed to start: %s\n",
                        startup.value(QStringLiteral("error")).toString().toUtf8().constData());
            return 2;
        }
        check(awaited(startup.value(QStringLiteral("props")).toMap(), "startup") == 0,
              "a fresh app awaits nothing");

        // No handler connected. This is what every native host did before
        // effect completion existed.
        const auto props = requestAwait(host);
        check(awaited(props, "unhandled") == 0,
              "an unanswered await is failed, not left outstanding");
        check(props.value(QStringLiteral("status")).toString().contains(
                  QStringLiteral("no host handler")),
              "the app is told why, rather than just waiting");
        check(props.value(QStringLiteral("count")).toInt() == 0,
              "a failed completion does not advance the app");
    }

    {
        // Its own state file: the host above persisted, and a shared path would
        // restore its count here and make the assertion read a number this host
        // never produced.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_B"));
        MosaicHost host;
        QObject::connect(&host, &MosaicHost::effectRequested,
            [&host](const QVariant &id, const QString &, const QVariant &, const QString &delivery) {
                if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
                host.completeEffect(id, QVariantMap{{QStringLiteral("ok"),
                    QVariantMap{{QStringLiteral("amount"), 5}}}});
            });

        const auto props = requestAwait(host);
        check(awaited(props, "handled") == 0, "an answered await is settled");
        check(props.value(QStringLiteral("count")).toInt() == 5,
              "the handler's value reached the app");
    }

    if (failures) {
        std::printf("\n%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("\nall checks passed\n");
    return 0;
}
