import unittest

from tests import support

ALANINE = 89.04768
SERINE = 90.0555


def get_target(precursor, product):
    return {"id": "alanine", "precursor": precursor, "product": product, "rt": 2.385, "range": 0.1}


@unittest.skipIf(support.library_problem, support.library_problem)
@unittest.skipIf(support.fixture_problem, support.fixture_problem)
class ChromatogramTargets(unittest.TestCase):
    def setUp(self):
        self.file = support.quantion.parse_ion(str(support.SRM_FIXTURE))
        self.addCleanup(self.file.dispose)

    def peaks_for(self, target):
        return support.quantion.find_transitions(self.file, [target])

    def test_a_target_finds_its_transition(self):
        row = self.peaks_for(get_target(ALANINE, ALANINE))[0]
        self.assertGreater(row["intensity"], 0.0)
        self.assertAlmostEqual(row["rt"], 2.385, delta=0.05)

    def test_a_wrong_product_finds_nothing(self):
        row = self.peaks_for(get_target(ALANINE, SERINE))[0]
        self.assertEqual(row["intensity"], 0.0)

    def test_a_missing_precursor_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            self.peaks_for({"id": "alanine", "product": ALANINE, "rt": 2.385, "range": 0.1})
        self.assertEqual(caught.exception.kind, "missing_argument")
        self.assertEqual(caught.exception.argument, "targets[0].precursor")
