import unittest

from tests import support

ALANINE = 89.04768
MODEL = support.REPO_ROOT / "core" / "tests" / "fixtures" / "shift_model.onnx"


@unittest.skipIf(support.library_problem, support.library_problem)
@unittest.skipIf(support.fixture_problem, support.fixture_problem)
class UnknownKeys(unittest.TestCase):
    def setUp(self):
        self.file = support.quantion.parse_ion(str(support.ION_FIXTURE))
        self.addCleanup(self.file.dispose)

    def get_error(self, call):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            call()
        self.assertEqual(caught.exception.kind, "unknown_option")
        return str(caught.exception)

    def test_a_target_with_an_unknown_key_is_refused(self):
        message = self.get_error(lambda: support.quantion.find_targets(self.file, [{"mz": ALANINE, "rt": 2.385, "rnage": 0.1}]))
        self.assertEqual(message, "find_targets: targets[0].rnage is not an option; the options are mz, rt, id, range")

    def test_a_transition_with_an_unknown_key_is_refused(self):
        transition = {"precursor": ALANINE, "product": ALANINE, "rt": 2.385, "range": 0.1, "prodcut": 1}
        message = self.get_error(lambda: support.quantion.find_transitions(self.file, [transition]))
        self.assertEqual(message, "find_transitions: targets[0].prodcut is not an option; the options are precursor, product, rt, range, id")

    def test_a_peak_with_an_unknown_key_is_refused(self):
        peak = {"height": 1, "center": 2, "fwhm": 0.1, "tail": 0, "hieght": 5}
        message = self.get_error(lambda: support.quantion.draw_peaks([peak]))
        self.assertEqual(message, "draw_peaks: params[0].hieght is not an option; the options are height, center, fwhm, tail, r2")

    def test_a_warp_target_with_an_unknown_key_is_refused(self):
        warp = support.quantion.load_warp(support.quantion.load_model(str(MODEL)))
        message = self.get_error(lambda: warp.predict_rts(self.file, [{"mz": ALANINE, "rt": 2.385, "rt_": 1}]))
        self.assertEqual(message, "predict_rts: targets[0].rt_ is not an option; the options are mz, rt, id, range")

    def test_a_range_with_an_unknown_key_is_refused(self):
        message = self.get_error(lambda: support.quantion.get_scans(self.file, rt={"from": 1, "too": 2}))
        self.assertEqual(message, "get_scans: rt.too is not an option; the options are from, to")

    def test_an_unknown_option_lists_the_options(self):
        message = self.get_error(lambda: support.quantion.calculate_baseline([1.0, 2.0, 3.0], {"lamda": 5}))
        self.assertEqual(message, "calculate_baseline: lamda is not an option; the options are lambda, max_iterations")


    def test_a_peak_without_tail_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.draw_peaks([{"height": 1, "center": 2, "fwhm": 0.1}])
        self.assertEqual(caught.exception.kind, "missing_argument")
        self.assertEqual(str(caught.exception), "draw_peaks: params[0].tail is required")

if __name__ == "__main__":
    unittest.main()
