// Drives the emitted Qt host against the conformance runtime.
//
// The Rust tests around this crate assert on the TEXT of the emitted host.
// That cannot tell you it compiles, let alone that it behaves -- and the defect
// this file exists for is behavioural: before the host completed effects, an
// `await` was dropped and the app waited forever with nothing reporting it.
#include "MosaicHost.h"
#include "MosaicPlatformEffects.h"
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
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
                if (answeredOne) return;   // answer the FIRST, ignore the second
                answeredOne = true;
                // `chain` makes the completion produce ANOTHER effect. Its id
                // lives only in the adopted update's `effects` list, so a host
                // that overwrites that map instead of accumulating drops it --
                // never emitted, never failed, pending forever.
                host.completeEffect(id, QVariantMap{{QStringLiteral("ok"),
                    QVariantMap{{QStringLiteral("amount"), 3},
                                {QStringLiteral("chain"), true}}}});
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
        const auto props = requestAwait(host);
        // NOT `check(true, ...)`, which is what this was and which cannot fail.
        // Surviving the recursion is the easy half; the guard must also
        // discharge what it gave up on, or it swaps a crash for an app that can
        // never snapshot again.
        check(awaited(props, "depth-bounded") == 0,
              "a bounded-out settle leaves nothing outstanding");
        const auto snap = host.snapshot();
        check(snap.isValid() && !snap.toMap().contains(QStringLiteral("error")),
              "snapshot still works after the nesting bound fires");
    }


    {
        // The handler answers BOTH effects of the batch, each chaining.
        //
        // One re-entrancy slot holding "the last update" silently discarded the
        // first answer's new effect: it existed only in a map nobody kept, so it
        // was never emitted, never failed, and pending forever -- which kills
        // snapshot and restore for the life of the process. Answering only one
        // effect cannot reach it, which is why every earlier version of this
        // file passed while it was broken.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_F"));
        MosaicHost host;
        int answers = 0;
        QObject::connect(&host, &MosaicHost::effectRequested,
            [&host, &answers](const QVariant &id, const QString &, const QVariant &,
                              const QString &delivery) {
                if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
                // Chain only the first two, or this never terminates.
                const bool chain = answers < 2;
                answers++;
                host.completeEffect(id, QVariantMap{{QStringLiteral("ok"),
                    QVariantMap{{QStringLiteral("amount"), 1},
                                {QStringLiteral("chain"), chain}}}});
            });

        QVariantMap event;
        event.insert(QStringLiteral("name"), QStringLiteral("requestEffectBatch"));
        event.insert(QStringLiteral("payload"), QVariantMap{});
        const auto props = host.handleEvent(event).value(QStringLiteral("props")).toMap();
        check(answers >= 2, "the handler answered both effects of the batch");
        check(awaited(props, "both-answered batch") == 0,
              "a fully-answered chaining batch leaves nothing outstanding");
        const auto snap = host.snapshot();
        check(snap.isValid() && !snap.toMap().contains(QStringLiteral("error")),
              "snapshot still works after a fully-answered chaining batch");
    }


    {
        // The shape a real file dialog needs, and the one that could not be
        // written before: the handler takes ownership and answers LATER.
        //
        // Previously the settle failed the effect the moment the handler
        // returned, so a dialog that had not closed yet lost its effect and the
        // late answer was rejected as already completed.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_G"));
        MosaicHost host;
        QVariant deferredId;
        QObject::connect(&host, &MosaicHost::effectRequested,
            [&host, &deferredId](const QVariant &id, const QString &, const QVariant &,
                                 const QString &delivery) {
                if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
                deferredId = id;
                host.deferEffect(id);       // "the dialog is open"
            });

        // The UI is told about updates it did not ask for.
        QVariantMap pushed;
        int pushes = 0;
        QObject::connect(&host, &MosaicHost::updated,
            [&pushed, &pushes](const QVariantMap &update) { pushed = update; pushes++; });

        const auto afterRequest = requestAwait(host);
        check(deferredId.isValid(), "the handler was offered the effect");
        check(awaited(afterRequest, "deferred") == 1,
              "a deferred effect stays outstanding rather than being failed");
        check(afterRequest.value(QStringLiteral("status")).toString()
                  != QStringLiteral("failed: no host handler answered effect 1"),
              "a deferred effect is not failed behind the handler's back");

        // Deferring something the runtime is NOT waiting on must be refused.
        // Ids are sequential and this is Q_INVOKABLE, so an off-by-one in QML
        // would otherwise turn the fail sweep off for an effect nothing will
        // ever answer -- wedging snapshot for the life of the process.
        const auto bogus = host.deferEffect(QVariant::fromValue(quint64{99999}));
        check(bogus.value(QStringLiteral("error")).toString().contains(
                  QStringLiteral("not awaiting")),
              "deferring an effect nothing awaits is refused");

        // ...the dialog closes, on whatever thread, whenever.
        host.completeEffect(deferredId, QVariantMap{{QStringLiteral("ok"),
            QVariantMap{{QStringLiteral("amount"), 9}}}});

        check(pushes == 1, "the late answer reached the UI as an update");
        const auto finalProps = pushed.value(QStringLiteral("props")).toMap();
        check(awaited(finalProps, "answered-late") == 0,
              "answering a deferred effect settles it");
        check(finalProps.value(QStringLiteral("count")).toInt() == 9,
              "the deferred answer's value reached the app");
        const auto snap = host.snapshot();
        check(snap.isValid() && !snap.toMap().contains(QStringLiteral("error")),
              "snapshot works again once the deferred effect is answered");
    }

    {
        // UI87 §7.4a: the platform library's router owns dispatch. The
        // conformance app's own kind (`conformance.counter`) is not standard,
        // so with no claimed kinds it must still reach a handler connected the
        // old way -- through the signal -- exactly once.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_H"));
        MosaicHost host;
        int signalled = 0;
        QObject::connect(&host, &MosaicHost::effectRequested, &host,
            [&host, &signalled](const QVariant &id, const QString &, const QVariant &,
                                const QString &delivery) {
                signalled++;
                if (delivery.compare(QStringLiteral("await"), Qt::CaseInsensitive) != 0) return;
                host.completeEffect(id, QVariantMap{{QStringLiteral("ok"),
                    QVariantMap{{QStringLiteral("amount"), 5}}}});
            }, Qt::DirectConnection);
        installMosaicPlatformEffects(host, std::nullopt, nullptr);
        installMosaicPlatformEffects(host, std::nullopt, nullptr); // idempotent
        const auto props = requestAwait(host);
        check(signalled == 1, "an app kind reaches the connected handler once");
        check(props.value(QStringLiteral("count")).toInt() == 5,
              "the connected handler's answer reached the app");
        check(awaited(props, "routed-app") == 0, "a routed app kind leaves nothing outstanding");

        // A second owner of one effect is refused.
        QVariant firstId;
        host.setEffectHandler([&host, &firstId](const QVariant &id, const QString &,
                                                const QVariant &, const QString &) {
            firstId = id;
            host.deferEffect(id);
        });
        requestAwait(host);
        const auto second = host.deferEffect(firstId);
        check(second.value(QStringLiteral("error")).toString().contains(
                  QStringLiteral("already owned")),
              "a second owner of a deferred effect is refused");
        host.completeEffect(firstId, QVariantMap{{QStringLiteral("cancelled"), QVariantMap{}}});
    }

    {
        // Claimed kinds that do not include the app's: nobody answers here,
        // the signal is not emitted, and the host fails the Await with a
        // reason rather than leaving it pending.
        qputenv("MOSAIC_APP_STATE_PATH", qgetenv("MOSAIC_PROBE_STATE_I"));
        MosaicHost host;
        int signalled = 0;
        QObject::connect(&host, &MosaicHost::effectRequested, &host,
            [&signalled](const QVariant &, const QString &, const QVariant &, const QString &) {
                signalled++;
            }, Qt::DirectConnection);
        installMosaicPlatformEffects(host, QSet<QString>{QStringLiteral("importAnki")}, nullptr);
        const auto props = requestAwait(host);
        check(signalled == 0, "an unclaimed non-standard kind reaches no handler");
        check(awaited(props, "routed-nobody") == 0 &&
                  props.value(QStringLiteral("status")).toString().contains(QStringLiteral("no host handler")),
              "an unclaimed non-standard kind is failed, not left pending");
    }

    {
        // The library's own behaviour, with fake dialogs: no display needed.
        struct FakeDialogs : MosaicFileDialogs {
            QString choice;
            int opened = 0;
            QStringList lastExtensions;
            QString chooseFileToOpen(const QStringList &extensions) override {
                opened++; lastExtensions = extensions; return choice;
            }
            QString chooseFileToSave(const QString &, const QStringList &extensions) override {
                opened++; lastExtensions = extensions; return choice;
            }
        };
        QTemporaryDir directory;
        check(directory.isValid(), "a scratch directory for the file checks");
        const auto path = [&directory](const char *name) { return directory.filePath(QString::fromUtf8(name)); };

        check(mosaicRoutesToPlatform(QStringLiteral("files.save"), QSet<QString>{QStringLiteral("files.save")}) == false &&
                  mosaicRoutesToPlatform(QStringLiteral("files.open"), QSet<QString>{QStringLiteral("importAnki")}) == true &&
                  mosaicRoutesToPlatform(QStringLiteral("files.save"), std::nullopt) == true &&
                  mosaicRoutesToPlatform(QStringLiteral("importAnki"), std::nullopt) == false &&
                  !mosaicRoutesToPlatform(QStringLiteral("other"), QSet<QString>{QStringLiteral("importAnki")}).has_value(),
              "the platform library routes by kind");

        FakeDialogs saving;
        saving.choice = path("journal.json");
        const auto saved = mosaicRunFilesSave(QVariantMap{
            {QStringLiteral("suggestedName"), QStringLiteral("journal.json")},
            {QStringLiteral("accept"), QVariantList{QStringLiteral("application/json")}},
            {QStringLiteral("bytes"), QString::fromLatin1(QByteArray("{\"v\":1}").toBase64())}}, saving);
        QFile savedFile(path("journal.json"));
        check(saved.value(QStringLiteral("ok")).toMap().value(QStringLiteral("name")).toString() == QStringLiteral("journal.json") &&
                  savedFile.open(QIODevice::ReadOnly) && savedFile.readAll() == QByteArray("{\"v\":1}"),
              "files.save writes the bytes under the chosen name");
        check(saving.lastExtensions == QStringList{QStringLiteral("json")}, "the save dialog filters to json");
        check(QDir(directory.path()).entryList(QStringList{QStringLiteral("*.tmp")}, QDir::Hidden | QDir::Files).isEmpty(),
              "files.save leaves no temporary file behind");

        bool allRefused = true;
        for (const QString &name : {QStringLiteral("../x.json"), QStringLiteral(".zshrc"), QStringLiteral("a:b.json"),
                                    QStringLiteral("trailing."), QStringLiteral("trailing "), QStringLiteral("line\u2028break.json"),
                                    QStringLiteral("Invoice.pdf ") + QString(QChar(0xFE00)) + QStringLiteral(" x.html"),
                                    QString(QChar(0xD800)) + QStringLiteral(".json")}) {
            FakeDialogs never;
            const auto outcome = mosaicRunFilesSave(QVariantMap{{QStringLiteral("suggestedName"), name},
                                                                {QStringLiteral("bytes"), QStringLiteral("AA==")}}, never);
            allRefused = allRefused && outcome.contains(QStringLiteral("failed")) && never.opened == 0;
        }
        check(allRefused, "files.save refuses names that are paths or disguises");
        FakeDialogs never;
        const auto launcher = mosaicRunFilesSave(QVariantMap{{QStringLiteral("suggestedName"), QStringLiteral("run.command")},
                                                             {QStringLiteral("bytes"), QStringLiteral("AA==")}}, never);
        check(launcher.value(QStringLiteral("failed")).toMap().value(QStringLiteral("message")).toString()
                  == QStringLiteral("suggestedName must not end in an executable extension") && never.opened == 0,
              "files.save refuses a launcher when no type is named");
        check(mosaicIsPlainFileName(QStringLiteral("\u2764\uFE0F list.txt")), "an emoji's own selector is fine");
        const auto notBase64 = mosaicRunFilesSave(QVariantMap{{QStringLiteral("suggestedName"), QStringLiteral("a.txt")},
                                                              {QStringLiteral("bytes"), QStringLiteral("%%%")}}, never);
        check(notBase64.contains(QStringLiteral("failed")), "files.save refuses bytes that are not base64");

        FakeDialogs cancelling;
        check(mosaicRunFilesSave(QVariantMap{{QStringLiteral("suggestedName"), QStringLiteral("a.txt")},
                                             {QStringLiteral("bytes"), QStringLiteral("AA==")}}, cancelling)
                  .contains(QStringLiteral("cancelled")),
              "a cancelled dialog is not a failure");

        QFile photo(path("photo.PNG"));
        photo.open(QIODevice::WriteOnly);
        photo.write(QByteArray("\x01\x02\x03", 3));
        photo.close();
        FakeDialogs opening;
        opening.choice = path("photo.PNG");
        const auto opened = mosaicRunFilesOpen(QVariantMap{{QStringLiteral("accept"),
            QVariantList{QStringLiteral("image/png")}}}, opening).value(QStringLiteral("ok")).toMap();
        check(opened.value(QStringLiteral("name")).toString() == QStringLiteral("photo.PNG") &&
                  opened.value(QStringLiteral("mimeType")).toString() == QStringLiteral("image/png") &&
                  opened.value(QStringLiteral("bytes")).toString() == QStringLiteral("AQID"),
              "files.open returns the name, type and bytes, never the path");
        FakeDialogs folder;
        folder.choice = directory.path();
        check(mosaicRunFilesOpen(QVariantMap{}, folder).value(QStringLiteral("failed")).toMap()
                  .value(QStringLiteral("message")).toString() == QStringLiteral("that is not a regular file"),
              "files.open refuses something that is not a regular file");
    }

    if (failures) {
        std::printf("\n%d check(s) failed\n", failures);
        return 1;
    }
    std::printf("\nall checks passed\n");
    return 0;
}
