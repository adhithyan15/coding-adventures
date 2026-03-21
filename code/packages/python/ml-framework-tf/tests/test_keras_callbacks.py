"""Tests for tf.keras.callbacks — training hooks."""

from ml_framework_tf.keras.callbacks import (
    Callback,
    EarlyStopping,
    History,
    LearningRateScheduler,
    ModelCheckpoint,
)


class TestHistory:
    def test_records_metrics(self):
        h = History()
        h.on_epoch_end(0, {"loss": 0.5, "accuracy": 0.8})
        h.on_epoch_end(1, {"loss": 0.3, "accuracy": 0.9})
        assert h.history["loss"] == [0.5, 0.3]
        assert h.history["accuracy"] == [0.8, 0.9]

    def test_empty_logs(self):
        h = History()
        h.on_epoch_end(0, None)
        assert h.history == {}

    def test_new_metric_added_later(self):
        h = History()
        h.on_epoch_end(0, {"loss": 0.5})
        h.on_epoch_end(1, {"loss": 0.3, "val_loss": 0.4})
        assert "val_loss" in h.history
        assert len(h.history["val_loss"]) == 1


class TestEarlyStopping:
    def test_no_stop_when_improving(self):
        es = EarlyStopping(patience=3)
        es.on_train_begin()
        es.on_epoch_end(0, {"val_loss": 0.5})
        es.on_epoch_end(1, {"val_loss": 0.4})
        es.on_epoch_end(2, {"val_loss": 0.3})
        assert not es.stop_training

    def test_stop_after_patience(self):
        es = EarlyStopping(patience=2)
        es.on_train_begin()
        es.on_epoch_end(0, {"val_loss": 0.5})
        es.on_epoch_end(1, {"val_loss": 0.6})
        es.on_epoch_end(2, {"val_loss": 0.7})
        assert es.stop_training
        assert es.stopped_epoch == 2

    def test_reset_on_improvement(self):
        es = EarlyStopping(patience=2)
        es.on_train_begin()
        es.on_epoch_end(0, {"val_loss": 0.5})
        es.on_epoch_end(1, {"val_loss": 0.6})  # no improvement
        es.on_epoch_end(2, {"val_loss": 0.3})  # improvement!
        es.on_epoch_end(3, {"val_loss": 0.4})  # no improvement
        assert not es.stop_training  # patience resets

    def test_min_delta(self):
        es = EarlyStopping(patience=2, min_delta=0.1)
        es.on_train_begin()
        es.on_epoch_end(0, {"val_loss": 0.5})
        es.on_epoch_end(1, {"val_loss": 0.45})  # improved but < min_delta
        assert not es.stop_training  # patience=2, wait=1, not stopped yet
        es.on_epoch_end(2, {"val_loss": 0.43})  # still < min_delta improvement
        assert es.stop_training  # wait=2 >= patience=2

    def test_missing_monitor(self):
        es = EarlyStopping(patience=1)
        es.on_train_begin()
        es.on_epoch_end(0, {"loss": 0.5})  # no val_loss
        assert not es.stop_training

    def test_none_logs(self):
        es = EarlyStopping()
        es.on_train_begin()
        es.on_epoch_end(0, None)
        assert not es.stop_training


class TestModelCheckpoint:
    def test_tracks_best(self):
        cp = ModelCheckpoint()
        cp.on_epoch_end(0, {"val_loss": 0.5})
        cp.on_epoch_end(1, {"val_loss": 0.3})
        cp.on_epoch_end(2, {"val_loss": 0.4})
        assert cp.best == 0.3
        assert cp.best_epoch == 1

    def test_no_monitor_key(self):
        cp = ModelCheckpoint()
        cp.on_epoch_end(0, {"loss": 0.5})
        assert cp.best is None

    def test_none_logs(self):
        cp = ModelCheckpoint()
        cp.on_epoch_end(0, None)
        assert cp.best is None


class TestLearningRateScheduler:
    def test_adjusts_lr(self):
        class MockOptimizer:
            learning_rate = 0.1

        opt = MockOptimizer()
        scheduler = LearningRateScheduler(lambda e, lr: lr * 0.5)
        scheduler.set_optimizer(opt)
        scheduler.on_epoch_begin(0)
        assert opt.learning_rate == 0.05

    def test_no_optimizer(self):
        scheduler = LearningRateScheduler(lambda e, lr: lr)
        scheduler.on_epoch_begin(0)  # should not crash

    def test_epoch_based_schedule(self):
        class MockOpt:
            learning_rate = 1.0

        opt = MockOpt()
        scheduler = LearningRateScheduler(
            lambda epoch, lr: lr if epoch < 2 else lr * 0.1
        )
        scheduler.set_optimizer(opt)
        scheduler.on_epoch_begin(0)
        assert opt.learning_rate == 1.0
        scheduler.on_epoch_begin(1)
        assert opt.learning_rate == 1.0
        scheduler.on_epoch_begin(2)
        assert abs(opt.learning_rate - 0.1) < 1e-6


class TestCallbackBase:
    def test_all_hooks_are_noop(self):
        cb = Callback()
        cb.on_train_begin()
        cb.on_train_end()
        cb.on_epoch_begin(0)
        cb.on_epoch_end(0)
