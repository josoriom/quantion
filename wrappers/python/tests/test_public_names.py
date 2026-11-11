import inspect
import unittest
import warnings

from tests import support


@unittest.skipIf(support.library_problem, support.library_problem)
class RenamedClass(unittest.TestCase):
    def test_the_new_name_is_a_class(self):
        self.assertTrue(inspect.isclass(support.quantion.Quantion))

    def test_the_new_name_is_advertised(self):
        self.assertIn("Quantion", support.quantion.__all__)

    def test_the_old_name_is_gone(self):
        self.assertFalse(hasattr(support.quantion, "MsUtils"))

    def test_the_new_name_does_not_warn(self):
        with warnings.catch_warnings(record=True) as seen:
            warnings.simplefilter("always")
            support.quantion.Quantion
        self.assertEqual(seen, [])

    def test_the_old_name_is_not_advertised(self):
        self.assertNotIn("MsUtils", support.quantion.__all__)

    def test_an_unknown_name_still_fails(self):
        with self.assertRaises(AttributeError):
            support.quantion.no_such_name


@unittest.skipIf(support.library_problem, support.library_problem)
class RemovedStderrCapture(unittest.TestCase):
    def test_the_bridge_has_no_capture_pair(self):
        self.assertFalse(hasattr(support._bridge, "start_stderr_capture"))
        self.assertFalse(hasattr(support._bridge, "stop_stderr_capture"))

    def test_the_package_has_no_capture_pair(self):
        self.assertFalse(hasattr(support.quantion, "start_stderr_capture"))
        self.assertFalse(hasattr(support.quantion, "stop_stderr_capture"))

    def test_the_capture_pair_is_not_advertised(self):
        self.assertNotIn("start_stderr_capture", support.quantion.__all__)
        self.assertNotIn("stop_stderr_capture", support.quantion.__all__)


@unittest.skipIf(support.library_problem, support.library_problem)
class PublicSurface(unittest.TestCase):
    def test_every_advertised_name_exists(self):
        for name in support.quantion.__all__:
            self.assertTrue(hasattr(support.quantion, name), name)

    def test_the_old_conversions_are_gone(self):
        for name in ("mzml_to_ion", "mzml_to_ion_file", "ion_to_mzml", "draw_peak"):
            self.assertFalse(hasattr(support.quantion, name), name)

    def test_every_option_type_and_enum_is_exported(self):
        for name in ("QuantionError", "TimeUnit", "PeakShape", "FindPeaksOptions", "CalculateEicOptions",
                     "WriteIonOptions", "Scan", "IonImage", "PeakParameters"):
            self.assertIn(name, support.quantion.__all__)

    def test_parse_ion_is_the_only_ion_reader(self):
        for name in ("parse_ion_path", "parse_ion_raw", "parse_ion_remote", "parse_ion_url"):
            self.assertFalse(hasattr(support.quantion, name), name)
            self.assertFalse(hasattr(support.quantion.Quantion, name), name)
