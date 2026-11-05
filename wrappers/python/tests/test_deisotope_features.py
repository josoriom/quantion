import unittest

from tests import support

ISOTOPE_DIFF = 1.003354835


def make_feature(mz, intensity, **extra):
    return {"mz": mz, "rt": 5.0, "from": 4.9, "to": 5.1, "intensity": intensity, "integral": intensity, "n_points": 9, **extra}


@unittest.skipIf(support.library_problem, support.library_problem)
class DeisotopeFeatures(unittest.TestCase):
    def test_removes_the_isotope_and_sorts_by_mz(self):
        features = [make_feature(300.0, 500.0), make_feature(200.0 + ISOTOPE_DIFF, 100.0), make_feature(200.0, 1000.0)]
        found = support.quantion.deisotope_features(features)
        self.assertEqual([feature["mz"] for feature in found], [200.0, 300.0])
        self.assertEqual(found[0]["n_points"], 9)
        self.assertNotIn("frequency", found[0])

    def test_keeps_frequency(self):
        found = support.quantion.deisotope_features([make_feature(200.0, 1000.0, frequency=0.5)])
        self.assertEqual(found[0]["frequency"], 0.5)

    def test_uses_the_options(self):
        features = [make_feature(200.0, 1000.0), make_feature(200.0 + ISOTOPE_DIFF + 0.003, 100.0)]
        self.assertEqual(len(support.quantion.deisotope_features(features)), 1)
        self.assertEqual(len(support.quantion.deisotope_features(features, {"mz_tolerance": 0.001})), 2)

    def test_accepts_an_empty_list(self):
        self.assertEqual(support.quantion.deisotope_features([]), [])

    def test_rejects_unknown_keys_and_invalid_values(self):
        with self.assertRaises(support.quantion.QuantionError):
            support.quantion.deisotope_features([], {"window": 1})
        with self.assertRaises(support.quantion.QuantionError):
            support.quantion.deisotope_features([], {"rt_tolerance": -1})
        with self.assertRaises(support.quantion.QuantionError):
            support.quantion.deisotope_features([make_feature(200.0, 1000.0, extra=1)])
        with self.assertRaises(support.quantion.QuantionError):
            support.quantion.deisotope_features([{"mz": 200.0}])
        with self.assertRaises(support.quantion.QuantionError):
            support.quantion.deisotope_features("features")
