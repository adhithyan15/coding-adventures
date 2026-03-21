"""Tests for the callbacks module."""

from ml_framework_keras.callbacks import (
    Callback,
    EarlyStopping,
    History,
    LearningRateScheduler,
    ModelCheckpoint,
)


class MockModel:
    """A minimal mock model for testing callbacks."""

    def __init__(self):
        self.trainable_weights = []

    class MockOptimizer:
        def __init__(self):
            self.learning_rate = 0.01

    _optimizer = MockOptimizer()


class TestCallback:
    def test_all_hooks_are_noop(self):
        cb = Callback()
        cb.on_train_begin({})
        cb.on_train_end({})
        cb.on_epoch_begin(0, {})
        cb.on_epoch_end(0, {})

    def test_hooks_accept_none_logs(self):
        cb = Callback()
        cb.on_train_begin(None)
        cb.on_epoch_end(0, None)


class TestHistory:
    def test_records_metrics(self):
        h = History()
        h.on_epoch_end(0, {"loss": 0.5, "accuracy": 0.8})
        h.on_epoch_end(1, {"loss": 0.3, "accuracy": 0.9})
        assert h.history["loss"] == [0.5, 0.3]
        assert h.history["accuracy"] == [0.8, 0.9]

    def test_records_epochs(self):
        h = History()
        h.on_epoch_end(0, {"loss": 0.5})
        h.on_epoch_end(1, {"loss": 0.3})
        assert h.epoch == [0, 1]

    def test_empty_logs(self):
        h = History()
        h.on_epoch_end(0, None)
        assert h.epoch == [0]

    def test_new_metrics_added(self):
        h = History()
        h.on_epoch_end(0, {"loss": 0.5})
        h.on_epoch_end(1, {"loss": 0.3, "val_loss": 0.4})
        assert h.history["loss"] == [0.5, 0.3]
        assert h.history["val_loss"] == [0.4]


class TestEarlyStopping:
    def test_stops_after_patience(self):
        es = EarlyStopping(patience=3)
        es.on_train_begin({})

        # Simulate loss not improving
        es.on_epoch_end(0, {"val_loss": 1.0})
        assert not es._stopped
        es.on_epoch_end(1, {"val_loss": 1.1})
        assert not es._stopped
        es.on_epoch_end(2, {"val_loss": 1.2})
        assert not es._stopped
        es.on_epoch_end(3, {"val_loss": 1.3})
        assert es._stopped

    def test_resets_on_improvement(self):
        es = EarlyStopping(patience=2)
        es.on_train_begin({})

        es.on_epoch_end(0, {"val_loss": 1.0})
        es.on_epoch_end(1, {"val_loss": 1.1})  # no improvement
        assert not es._stopped
        es.on_epoch_end(2, {"val_loss": 0.5})  # improvement!
        assert es._wait == 0
        es.on_epoch_end(3, {"val_loss": 0.6})  # no improvement
        assert not es._stopped
        es.on_epoch_end(4, {"val_loss": 0.7})  # patience exhausted
        assert es._stopped

    def test_custom_monitor(self):
        es = EarlyStopping(monitor="loss", patience=1)
        es.on_train_begin({})
        es.on_epoch_end(0, {"loss": 1.0})
        es.on_epoch_end(1, {"loss": 1.1})
        assert es._stopped

    def test_missing_monitor_no_crash(self):
        es = EarlyStopping(monitor="val_loss", patience=1)
        es.on_train_begin({})
        es.on_epoch_end(0, {"loss": 1.0})  # val_loss not present
        assert not es._stopped

    def test_restore_best_weights(self):
        es = EarlyStopping(patience=1, restore_best_weights=True)
        mock_model = MockModel()

        # Create mock params
        class MockParam:
            def __init__(self, data):
                self.data = list(data)

        p1 = MockParam([1.0, 2.0])
        mock_model.trainable_weights = [p1]
        es._model = mock_model
        es.on_train_begin({})

        es.on_epoch_end(0, {"val_loss": 0.5})
        assert es._best_weights == [[1.0, 2.0]]

        p1.data = [3.0, 4.0]  # weights changed
        es.on_epoch_end(1, {"val_loss": 0.6})  # worse
        es.on_epoch_end(2, {"val_loss": 0.7})  # stopped
        assert es._stopped
        assert p1.data == [1.0, 2.0]  # restored!

    def test_min_delta(self):
        es = EarlyStopping(patience=1, min_delta=0.1)
        es.on_train_begin({})
        es.on_epoch_end(0, {"val_loss": 1.0})
        es.on_epoch_end(1, {"val_loss": 0.95})  # improved by 0.05 < min_delta
        assert es._wait == 1  # not counted as improvement

    def test_train_begin_resets(self):
        es = EarlyStopping(patience=1)
        es._stopped = True
        es._wait = 5
        es.on_train_begin({})
        assert not es._stopped
        assert es._wait == 0


class TestModelCheckpoint:
    def test_saves_on_improvement(self):
        mc = ModelCheckpoint(save_best_only=True)

        class MockParam:
            def __init__(self, data):
                self.data = list(data)

        mock_model = MockModel()
        p = MockParam([1.0])
        mock_model.trainable_weights = [p]
        mc._model = mock_model

        mc.on_epoch_end(0, {"val_loss": 1.0})
        assert mc._saved_weights == [[1.0]]

        p.data = [2.0]
        mc.on_epoch_end(1, {"val_loss": 0.5})  # better
        assert mc._saved_weights == [[2.0]]

        p.data = [3.0]
        mc.on_epoch_end(2, {"val_loss": 0.8})  # worse
        assert mc._saved_weights == [[2.0]]  # not updated

    def test_saves_every_epoch_when_not_best_only(self):
        mc = ModelCheckpoint(save_best_only=False)

        class MockParam:
            def __init__(self, data):
                self.data = list(data)

        mock_model = MockModel()
        p = MockParam([1.0])
        mock_model.trainable_weights = [p]
        mc._model = mock_model

        mc.on_epoch_end(0, {"val_loss": 1.0})
        p.data = [2.0]
        mc.on_epoch_end(1, {"val_loss": 2.0})  # worse but still saves
        assert mc._saved_weights == [[2.0]]

    def test_no_monitor_metric(self):
        mc = ModelCheckpoint(save_best_only=False)
        mock_model = MockModel()
        mc._model = mock_model
        mc.on_epoch_end(0, {"loss": 1.0})  # val_loss not present


class TestLearningRateScheduler:
    def test_adjusts_lr(self):
        def schedule(epoch, lr):
            return lr * 0.5

        lrs = LearningRateScheduler(schedule)
        mock_model = MockModel()
        lrs._model = mock_model

        lrs.on_epoch_begin(0, {})
        assert mock_model._optimizer.learning_rate == 0.005

        lrs.on_epoch_begin(1, {})
        assert abs(mock_model._optimizer.learning_rate - 0.0025) < 1e-10

    def test_step_schedule(self):
        def schedule(epoch, lr):
            if epoch < 5:
                return 0.01
            return 0.001

        lrs = LearningRateScheduler(schedule)
        mock_model = MockModel()
        lrs._model = mock_model

        lrs.on_epoch_begin(0, {})
        assert mock_model._optimizer.learning_rate == 0.01
        lrs.on_epoch_begin(5, {})
        assert mock_model._optimizer.learning_rate == 0.001

    def test_no_model_no_crash(self):
        lrs = LearningRateScheduler(lambda e, lr: lr)
        lrs.on_epoch_begin(0, {})  # no crash
