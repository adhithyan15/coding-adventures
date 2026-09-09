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


    {
        // A batch where the handler answers ONE effect and ignores the other.
        //
        // Treating "a handler answered" and "an effect went unanswered" as
        // alternatives drops the ignored one: never failed, never cleared, and
        // the runtime then refuses to snapshot or restore for the rest of the
        // process, because both are gated on nothing being pending. Every
        // single-effect test passes while that is broken.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_C"));
        MosaicHost host;
        bool answeredOne = false;
        QObject::connect(&host, &MosaicHost::effectRequested,
            [&host, &answeredOne](const QVariant &id, const QString &, const QVariant &,
                                  const QString &delivery) {
                if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
                if (answeredOne) return;   // ignore the first, answer the second
                answeredOne = true;
                host.completeEffect(id, QVariantMap{{QStringLiteral("ok"),
                    QVariantMap{{QStringLiteral("amount"), 3}}}});
            });

        QVariantMap event;
        event.insert(QStringLiteral("name"), QStringLiteral("requestEffectBatch"));
        event.insert(QStringLiteral("payload"), QVariantMap{});
        const auto props = host.handleEvent(event).value(QStringLiteral("props")).toMap();
        check(awaited(props, "mixed batch") == 0,
              "a partly-answered batch leaves nothing outstanding");

        // The proof it is really settled: the runtime refuses both of these
        // while anything is pending.
        const auto snap = host.snapshot();
        check(snap.isValid() && !snap.toMap().contains(QStringLiteral("error")),
              "snapshot still works after a partly-answered batch");
    }

    {
        // An id QML could plausibly produce. `toULongLong` accepts 3.5 and
        // truncates it to 4, which would answer a different outstanding effect.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_D"));
        MosaicHost host;
        // "an error came back" is NOT the assertion: an id that truncates to 4
        // is rejected by the RUNTIME as unknown, so a host with no validation
        // at all still produces an error here. The host's own message is what
        // distinguishes rejecting the id from forwarding a corrupted one.
        const auto refused = host.completeEffect(QVariant(3.5),
            QVariantMap{{QStringLiteral("cancelled"), QVariantMap{}}});
        check(refused.value(QStringLiteral("error")).toString().contains(
                  QStringLiteral("non-negative integer")),
              "a non-integral effect id is refused, not truncated");
        const auto negative = host.completeEffect(QVariant(-1),
            QVariantMap{{QStringLiteral("cancelled"), QVariantMap{}}});
        check(negative.value(QStringLiteral("error")).toString().contains(
                  QStringLiteral("non-negative integer")),
              "a negative effect id is refused, not wrapped");
    }

    {
        // A handler that calls back into the host instead of answering. The
        // round bound does not bound NESTING; without a depth bound this
        // recurses until the stack gives out.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_E"));
        MosaicHost host;
        QObject::connect(&host, &MosaicHost::effectRequested,
            [&host](const QVariant &, const QString &, const QVariant &, const QString &) {
                QVariantMap again;
                again.insert(QStringLiteral("name"), QStringLiteral("requestEffect"));
                again.insert(QStringLiteral("payload"),
                             QVariantMap{{QStringLiteral("notify"), false}});
                host.handleEvent(again);
            });
        const auto result = requestAwait(host);
        check(true, "a handler that re-enters the host does not crash it");
        (void)result;
    }

    if (failures) {
        std::printf("\n%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("\nall checks passed\n");
    return 0;
}
