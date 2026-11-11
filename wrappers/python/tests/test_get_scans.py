import base64
import unittest

import numpy as np

from tests import support


@unittest.skipIf(support.library_problem, support.library_problem)
@unittest.skipIf(support.fixture_problem, support.fixture_problem)
class ScanQueries(unittest.TestCase):
    def setUp(self):
        self.file = support.quantion.parse_ion(str(support.ION_FIXTURE))
        self.addCleanup(self.file.dispose)

    def test_an_rt_range_gives_every_scan_inside_it(self):
        scans = support.quantion.get_scans(self.file, rt={"from": 2.3, "to": 2.5})
        self.assertGreater(len(scans), 0)
        self.assertTrue(all(2.3 <= scan["rt"] <= 2.5 for scan in scans))

    def test_one_rt_gives_a_list_with_the_closest_scan(self):
        scans = support.quantion.get_scans(self.file, rt=2.385)
        self.assertEqual(len(scans), 1)
        self.assertAlmostEqual(scans[0]["rt"], 2.385, delta=0.05)

    def test_seconds_find_the_same_scan(self):
        in_minutes = support.quantion.get_scans(self.file, rt=2.385)
        in_seconds = support.quantion.get_scans(self.file, rt=2.385 * 60, options={"time_unit": "SECONDS"})
        self.assertEqual(in_minutes[0]["rt"], in_seconds[0]["rt"])

    def test_a_scan_is_flat(self):
        scan = support.quantion.get_scans(self.file, rt=2.385)[0]
        self.assertEqual(
            list(scan),
            ["index", "rt", "mz", "intensity", "level", "precursor_mz", "base_peak_mz", "base_peak_intensity",
             "total_ion_current", "polarity", "x", "y", "z"],
        )
        self.assertEqual(scan["level"], 1)

    def test_a_query_with_neither_rt_nor_mz_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.get_scans(self.file)
        self.assertEqual(caught.exception.argument, "rt")

    def test_a_range_without_to_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.get_scans(self.file, rt={"from": 2.3})
        self.assertEqual(caught.exception.kind, "missing_argument")
        self.assertEqual(caught.exception.argument, "rt.to")

    def test_the_old_tuple_names_are_gone(self):
        with self.assertRaises(TypeError):
            support.quantion.get_scans(self.file, rt_range=(2.3, 2.5))


FINGERPRINT = {"key": "fingerprint", "accession": "MS:1000786"}
FINGERPRINT_LENGTH = 256


def get_fingerprint(position):
    return (np.arange(FINGERPRINT_LENGTH, dtype=np.float64) - 128.0) / 256.0 + position


def write_array(accession, name, value, values):
    encoded = base64.b64encode(np.asarray(values, dtype=np.float64).tobytes()).decode()
    return (
        f'<binaryDataArray arrayLength="{len(values)}" encodedLength="{len(encoded)}">'
        '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float" value=""/>'
        '<cvParam cvRef="MS" accession="MS:1000576" name="no compression" value=""/>'
        f'<cvParam cvRef="MS" accession="{accession}" name="{name}" value="{value}"/>'
        f"<binary>{encoded}</binary></binaryDataArray>"
    )


def write_spectrum(position):
    return (
        f'<spectrum index="{position}" id="scan={position + 1}" defaultArrayLength="3">'
        '<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/>'
        '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
        f'<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{position + 1}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList>'
        f'<precursorList count="1"><precursor><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{300 + position}"/></selectedIon></selectedIonList></precursor></precursorList>'
        '<binaryDataArrayList count="3">'
        + write_array("MS:1000514", "m/z array", "", [100.5, 200.25, 300.125])
        + write_array("MS:1000515", "intensity array", "", [10.0, 20.0, 30.0])
        + write_array("MS:1000786", "non-standard data array", "ms2_fingerprint", get_fingerprint(position))
        + "</binaryDataArrayList></spectrum>"
    )


def write_mzml():
    spectra = "".join(write_spectrum(position) for position in range(3))
    return (
        '<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
        '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r">'
        f'<spectrumList count="3" defaultDataProcessingRef="d">{spectra}</spectrumList></run></mzML>'
    ).encode()


@unittest.skipIf(support.library_problem, support.library_problem)
class ScanArrays(unittest.TestCase):
    def setUp(self):
        mzml = support.quantion.parse_mzml(write_mzml())
        self.addCleanup(mzml.dispose)
        self.bytes = support.quantion.write_ion(mzml)
        self.file = support.quantion.parse_ion(self.bytes)
        self.addCleanup(self.file.dispose)
        self.whole_run = {"from": 0.0, "to": 10.0}

    def test_default_arrays_give_mz_and_intensity_and_no_others(self):
        scans = support.quantion.get_scans(self.file, rt=self.whole_run, options={"level": 2})
        self.assertEqual(len(scans), 3)
        self.assertEqual(scans[0]["mz"].tolist(), [100.5, 200.25, 300.125])
        self.assertEqual(scans[0]["intensity"].tolist(), [10.0, 20.0, 30.0])
        self.assertNotIn("fingerprint", scans[0])

    def test_custom_array_reads_back_exactly_with_negatives(self):
        options = {"level": 2, "arrays": [FINGERPRINT]}
        scans = support.quantion.get_scans(self.file, rt=self.whole_run, options=options)
        self.assertEqual(len(scans), 3)
        for position, scan in enumerate(scans):
            np.testing.assert_array_equal(scan["fingerprint"], get_fingerprint(position))
            self.assertEqual(len(scan["mz"]), 0)
            self.assertEqual(len(scan["intensity"]), 0)
            self.assertEqual(scan["precursor_mz"], 300 + position)
        self.assertTrue((scans[0]["fingerprint"] < 0).any())

    def test_custom_array_can_come_with_mz_and_intensity(self):
        arrays = [{"key": "mz", "accession": "MS:1000514"}, {"key": "intensity", "accession": "MS:1000515"}, FINGERPRINT]
        scans = support.quantion.get_scans(self.file, mz={"from": 300.5, "to": 301.5}, options={"level": 2, "arrays": arrays})
        self.assertEqual(len(scans), 1)
        self.assertEqual(scans[0]["mz"].tolist(), [100.5, 200.25, 300.125])
        np.testing.assert_array_equal(scans[0]["fingerprint"], get_fingerprint(1))

    def get_rts(self, rt, mz):
        return [scan["rt"] for scan in support.quantion.get_scans(self.file, rt=rt, mz=mz, options={"level": 2})]

    def test_rt_range_and_mz_range_keep_scans_inside_both(self):
        self.assertEqual(self.get_rts({"from": 0.5, "to": 2.5}, {"from": 300.5, "to": 302.5}), [2.0])

    def test_rt_range_and_closest_mz_give_one_scan_inside_the_range(self):
        self.assertEqual(self.get_rts({"from": 1.5, "to": 3.5}, 300.0), [2.0])

    def test_closest_rt_and_mz_range_give_the_nearest_scan_at_that_precursor(self):
        self.assertEqual(self.get_rts(1.0, {"from": 300.5, "to": 302.5}), [2.0])

    def test_closest_rt_and_closest_mz_pick_the_closest_precursor_first(self):
        self.assertEqual(self.get_rts(3.0, 300.2), [1.0])

    def test_mzml_gives_the_same_arrays(self):
        file = support.quantion.parse_mzml(write_mzml())
        self.addCleanup(file.dispose)
        scans = support.quantion.get_scans(file, rt=self.whole_run, options={"level": 2, "arrays": [FINGERPRINT]})
        np.testing.assert_array_equal(scans[2]["fingerprint"], get_fingerprint(2))

    def test_bad_arrays_are_refused(self):
        bad = [
            [],
            "mz",
            [{"key": "", "accession": "MS:1000786"}],
            [{"key": "rt", "accession": "MS:1000786"}],
            [{"key": "fingerprint", "accession": "1000786"}],
            [{"key": "mz", "accession": "MS:1000786"}],
            [{"key": "masses", "accession": "MS:1000514"}],
            [FINGERPRINT, FINGERPRINT],
            [{"key": "fingerprint"}],
            [{"key": "fingerprint", "accession": "MS:1000786", "name": "x"}],
        ]
        for arrays in bad:
            with self.subTest(arrays=arrays):
                with self.assertRaises(support.quantion.QuantionError):
                    support.quantion.get_scans(self.file, rt=self.whole_run, options={"arrays": arrays})
