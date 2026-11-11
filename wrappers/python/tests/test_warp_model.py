import math
import unittest

from tests import support

MODEL = support.REPO_ROOT / "core" / "tests" / "fixtures" / "shift_model.onnx"
EMPTY_FILE = support.REPO_ROOT / "core" / "tests" / "fixtures" / "test.ion"
TARGETS = [
    {"id": "first", "mz": 250.1, "rt": 1.2},
    {"id": "second", "mz": 310.2, "rt": 3.1},
]


@unittest.skipIf(support.library_problem, support.library_problem)
@unittest.skipIf(support.fixture_problem, support.fixture_problem)
class WarpModel(unittest.TestCase):
    def setUp(self):
        self.file = support.quantion.parse_ion(str(support.ION_FIXTURE))
        self.addCleanup(self.file.dispose)

    def load(self, source):
        model = support.quantion.load_model(source)
        self.addCleanup(model.dispose)
        warp = support.quantion.load_warp(model)
        self.addCleanup(warp.dispose)
        return warp

    def test_one_prediction_per_target_in_order(self):
        predicted = self.load(str(MODEL)).predict_rts(self.file, TARGETS)
        self.assertEqual(len(predicted), len(TARGETS))
        for prediction in predicted:
            self.assertEqual(set(prediction), {"rt", "uncertainty"})
            self.assertTrue(math.isfinite(prediction["rt"]))

    def test_a_model_from_bytes_predicts_the_same(self):
        from_path = self.load(str(MODEL)).predict_rts(self.file, TARGETS)
        from_bytes = self.load(MODEL.read_bytes()).predict_rts(self.file, TARGETS)
        self.assertEqual(from_path, from_bytes)

    def test_a_model_from_a_url_predicts_the_same(self):
        server = support.RangeServer(MODEL)
        self.addCleanup(server.stop)
        from_path = self.load(str(MODEL)).predict_rts(self.file, TARGETS)
        from_url = self.load(server.url).predict_rts(self.file, TARGETS)
        self.assertEqual(from_path, from_url)

    def test_a_remote_file_predicts_like_a_local_one(self):
        server = support.RangeServer(support.ION_FIXTURE)
        self.addCleanup(server.stop)
        remote = support.quantion.parse_ion(server.url)
        self.addCleanup(remote.dispose)
        warp = self.load(str(MODEL))
        local = warp.predict_rts(self.file, TARGETS)
        self.assertEqual(warp.predict_rts(remote, TARGETS), local)

    def test_a_target_without_mz_is_named_by_its_index(self):
        targets = [TARGETS[0], {"id": "broken", "rt": 2.0}]
        with self.assertRaises(support.quantion.QuantionError) as caught:
            self.load(str(MODEL)).predict_rts(self.file, targets)
        self.assertEqual(caught.exception.argument, "targets[1].mz")

    def test_a_target_with_a_bad_mz_is_named_by_rust(self):
        targets = [TARGETS[0], {"id": "broken", "mz": float("nan"), "rt": 2.0}]
        with self.assertRaisesRegex(support.quantion.QuantionError, "predict_rts: target 2 has no valid m/z or rt"):
            self.load(str(MODEL)).predict_rts(self.file, targets)

    def test_a_file_without_scans_in_the_model_range_is_refused(self):
        empty = support.quantion.parse_ion(str(EMPTY_FILE))
        self.addCleanup(empty.dispose)
        with self.assertRaisesRegex(support.quantion.QuantionError, "predict_rts: the file has no scans, the warp model needs"):
            self.load(str(MODEL)).predict_rts(empty, TARGETS)

    def test_keys_and_values_come_from_the_model(self):
        warp = self.load(str(MODEL))
        self.assertEqual(dict(zip(warp.get_keys(), warp.get_values()))["points"], "64")
        self.assertEqual(warp.get_value("points"), "64")
        self.assertIsNone(warp.get_value("missing"))

    def test_calibrate_with_one_sample_moves_each_rt_by_its_predicted_shift(self):
        warp = self.load(str(MODEL))
        predicted = warp.predict_rts(self.file, TARGETS)
        calibrated = warp.calibrate([self.file], [TARGETS])
        for target, prediction, reference in zip(TARGETS, predicted, calibrated):
            self.assertEqual(reference["id"], target["id"])
            self.assertEqual(reference["rt"], 2.0 * target["rt"] - prediction["rt"])

    def test_calibrate_takes_the_median_over_the_samples(self):
        warp = self.load(str(MODEL))
        early = [{**TARGETS[0], "rt": TARGETS[0]["rt"] - 0.2}, TARGETS[1]]
        late = [{**TARGETS[0], "rt": TARGETS[0]["rt"] + 0.2}, TARGETS[1]]
        middle = warp.calibrate([self.file], [TARGETS])
        files = [self.file, self.file, self.file]
        self.assertEqual(warp.calibrate(files, [early, TARGETS, late]), middle)

    def test_calibrate_needs_one_target_list_per_sample(self):
        with self.assertRaisesRegex(support.quantion.QuantionError, "got 1 samples and 2 target lists"):
            self.load(str(MODEL)).calibrate([self.file], [TARGETS, TARGETS])

    def test_calibrate_needs_an_id_for_each_target(self):
        targets = [TARGETS[0], {"mz": 310.2, "rt": 3.1}]
        with self.assertRaisesRegex(support.quantion.QuantionError, "calibrate: target 2 in list 1 has no id"):
            self.load(str(MODEL)).calibrate([self.file], [targets])

    def test_a_missing_model_is_refused(self):
        with self.assertRaisesRegex(support.quantion.QuantionError, "load_model: the file could not be read: the model was not read"):
            support.quantion.load_model(str(MODEL.with_name("missing.onnx")))

    def test_the_warp_keeps_working_after_the_model_is_disposed(self):
        model = support.quantion.load_model(str(MODEL))
        warp = support.quantion.load_warp(model)
        self.addCleanup(warp.dispose)
        model.dispose()
        self.assertEqual(len(warp.predict_rts(self.file, TARGETS)), len(TARGETS))
        self.assertEqual(warp.get_value("points"), "64")

    def test_the_model_keeps_working_after_the_warp_is_made(self):
        model = support.quantion.load_model(str(MODEL))
        self.addCleanup(model.dispose)
        warp = support.quantion.load_warp(model)
        warp.dispose()
        self.assertEqual(model.get_value("points"), "64")
        self.assertEqual(model.get_keys(), self.load(str(MODEL)).get_keys())

    def test_load_warp_needs_a_model(self):
        with self.assertRaisesRegex(support.quantion.QuantionError, "load_warp: model must be a Model from load_model"):
            support.quantion.load_warp(str(MODEL))
