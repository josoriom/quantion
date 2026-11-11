import unittest

from tests import support

MASS = 89.04768
TARGET_RT = 2.385


@unittest.skipIf(support.library_problem, support.library_problem)
@unittest.skipIf(support.fixture_problem, support.fixture_problem)
class FitAndDraw(unittest.TestCase):
    def setUp(self):
        file = support.quantion.parse_ion(str(support.ION_FIXTURE))
        self.addCleanup(file.dispose)
        self.eic = support.quantion.calculate_eic(file, MASS, {"from": 0.0, "to": 5.0})

    def fit(self, options=None):
        return support.quantion.fit_peak(self.eic["x"], self.eic["y"], TARGET_RT, options)

    def test_a_gaussian_fit_has_no_tail(self):
        self.assertEqual(self.fit({"shape": "GAUSSIAN"})["tail"], 0.0)

    def test_an_emg_fit_has_a_tail(self):
        self.assertGreater(self.fit()["tail"], 0.0)

    def test_the_fit_has_no_shape_field(self):
        self.assertNotIn("shape", self.fit())

    def test_the_fit_has_its_fields_and_r2(self):
        self.assertEqual(sorted(self.fit()), ["center", "fwhm", "height", "r2", "tail"])

    def test_peaks_have_only_the_public_fields(self):
        peaks = support.quantion.find_peaks(self.eic["x"], self.eic["y"])
        self.assertGreater(len(peaks), 0)
        self.assertEqual(sorted(peaks[0]), ["from", "integral", "intensity", "n_points", "rt", "to"])

    def test_the_enum_and_the_text_give_the_same_fit(self):
        self.assertEqual(self.fit({"shape": support.quantion.PeakShape.GAUSSIAN}), self.fit({"shape": "GAUSSIAN"}))

    def test_a_lower_case_shape_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            self.fit({"shape": "emg"})
        self.assertEqual(caught.exception.argument, "shape")

    def test_no_peak_gives_none(self):
        flat = [0.0] * len(self.eic["x"])
        self.assertIsNone(support.quantion.fit_peak(self.eic["x"], flat, TARGET_RT))

    def test_a_fit_can_be_drawn(self):
        chromatogram = support.quantion.draw_peaks([self.fit()], {"from": 0.0, "to": 5.0, "points": 500})
        self.assertEqual(len(chromatogram["x"]), 500)
        self.assertEqual(len(chromatogram["y"]), 500)


@unittest.skipIf(support.library_problem, support.library_problem)
class DrawPeaks(unittest.TestCase):
    def peak(self, center, height):
        return {"height": height, "center": center, "fwhm": 0.05, "tail": 0.0}

    def test_overlapping_peaks_add_up(self):
        drawn = support.quantion.draw_peaks(
            [self.peak(2.0, 100.0), self.peak(2.0, 50.0)], {"from": 1.5, "to": 2.5, "points": 101})
        self.assertAlmostEqual(max(drawn["y"]), 150.0, places=6)

    def test_the_range_fits_the_peaks_when_not_given(self):
        drawn = support.quantion.draw_peaks([self.peak(2.0, 100.0), self.peak(3.0, 100.0)])
        self.assertEqual(len(drawn["x"]), 1000)
        self.assertLess(drawn["x"][0], 2.0)
        self.assertGreater(drawn["x"][-1], 3.0)

    def test_a_missing_tail_is_named(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.draw_peaks([{"height": 1.0, "center": 2.0, "fwhm": 0.05}])
        self.assertEqual(caught.exception.kind, "missing_argument")
        self.assertEqual(caught.exception.argument, "params[0].tail")
