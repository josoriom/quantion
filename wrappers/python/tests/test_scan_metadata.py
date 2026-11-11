import base64
import tempfile
import unittest
from pathlib import Path

import numpy as np

from tests import support

SCAN_COUNT = 6


def get_level(position):
    return 1 if position % 2 == 0 else 2


def write_array(accession, name):
    encoded = base64.b64encode(np.array([100.0, 200.0], dtype=np.float64).tobytes()).decode()
    return (
        f'<binaryDataArray encodedLength="{len(encoded)}">'
        '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>'
        '<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>'
        f'<cvParam cvRef="MS" accession="{accession}" name="{name}"/>'
        f"<binary>{encoded}</binary></binaryDataArray>"
    )


MZ_UNIT = 'unitCvRef="MS" unitAccession="MS:1000040" unitName="m/z"'


def write_precursor(position):
    if get_level(position) == 1:
        return ""
    mz = 300 + position
    return (
        f'<precursorList count="1"><precursor spectrumRef="scan={position}"><isolationWindow>'
        f'<cvParam cvRef="MS" accession="MS:1000827" name="isolation window target m/z" value="{mz}" {MZ_UNIT}/>'
        '</isolationWindow><selectedIonList count="2"><selectedIon>'
        f'<cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{mz}" {MZ_UNIT}/>'
        '</selectedIon><selectedIon><cvParam cvRef="MS" accession="MS:1000041" name="charge state" value="2"/>'
        '</selectedIon></selectedIonList><activation>'
        '<cvParam cvRef="MS" accession="MS:1000133" name="collision-induced dissociation"/>'
        "</activation></precursor></precursorList>"
    )


def write_spectrum(position):
    level = get_level(position)
    start = position + 1
    return (
        f'<spectrum index="{position}" id="scan={start}" defaultArrayLength="2" dataProcessingRef="synthetic_processing">'
        '<referenceableParamGroupRef ref="common_params"/>'
        f'<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="{level}"/>'
        '<cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>'
        '<cvParam cvRef="MS" accession="MS:1000130" name="positive scan" value=""/>'
        f'<userParam name="library_id" value="SYNTHETIC{position:03}"/>'
        f'<userParam name="formula" value="C{start}H{level}"/>'
        '<userParam name="library_id" value="REPEATED"/>'
        '<scanList count="1"><cvParam cvRef="MS" accession="MS:1000795" name="no combination"/>'
        '<scan instrumentConfigurationRef="synthetic_instrument">'
        f'<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{start}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>'
        '<userParam name="scan_note" value="synthetic note"/>'
        '<scanWindowList count="1"><scanWindow>'
        f'<cvParam cvRef="MS" accession="MS:1000501" name="scan window lower limit" value="50" {MZ_UNIT}/>'
        "</scanWindow></scanWindowList></scan></scanList>"
        + write_precursor(position)
        + '<binaryDataArrayList count="2">'
        + write_array("MS:1000514", "m/z array")
        + write_array("MS:1000515", "intensity array")
        + "</binaryDataArrayList></spectrum>"
    )


def write_mzml():
    spectra = "".join(write_spectrum(position) for position in range(SCAN_COUNT))
    return (
        '<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
        '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList>'
        '<referenceableParamGroupList count="1"><referenceableParamGroup id="common_params">'
        '<userParam name="group_note" value="shared"/></referenceableParamGroup></referenceableParamGroupList><run id="r">'
        f'<spectrumList count="{SCAN_COUNT}" defaultDataProcessingRef="d">{spectra}</spectrumList></run></mzML>'
    ).encode()


def row(path, kind, accession, name, value, unit=None):
    return {
        "path": path,
        "kind": kind,
        "accession": accession,
        "name": name,
        "value": value,
        "unit": unit[0] if unit else None,
        "unit_accession": unit[1] if unit else None,
    }


def expected_rows(position):
    level = str(get_level(position))
    start = str(position + 1)
    mz = str(300 + position)
    scan = "spectrum/scanList/scan[0]"
    mz_unit = ("m/z", "MS:1000040")
    rows = [
        row("spectrum", "ATTRIBUTE", None, "index", str(position)),
        row("spectrum", "ATTRIBUTE", None, "id", f"scan={start}"),
        row("spectrum", "ATTRIBUTE", None, "defaultArrayLength", "2"),
        row("spectrum", "ATTRIBUTE", None, "dataProcessingRef", "synthetic_processing"),
        row("spectrum/referenceableParamGroupRef[0]", "ATTRIBUTE", None, "ref", "common_params"),
        row("spectrum", "CV", "MS:1000511", "ms level", level),
        row("spectrum", "CV", "MS:1000127", "centroid spectrum", None),
        row("spectrum", "CV", "MS:1000130", "positive scan", None),
        row("spectrum", "USER", None, "library_id", f"SYNTHETIC{position:03}"),
        row("spectrum", "USER", None, "formula", f"C{start}H{level}"),
        row("spectrum", "USER", None, "library_id", "REPEATED"),
        row("spectrum/scanList", "CV", "MS:1000795", "no combination", None),
        row(scan, "ATTRIBUTE", None, "instrumentConfigurationRef", "synthetic_instrument"),
        row(scan, "CV", "MS:1000016", "scan start time", start, ("minute", "UO:0000031")),
        row(scan, "USER", None, "scan_note", "synthetic note"),
        row(f"{scan}/scanWindowList/scanWindow[0]", "CV", "MS:1000501", "scan window lower limit", "50", mz_unit),
    ]
    if level == "2":
        precursor = "spectrum/precursorList/precursor[0]"
        rows += [
            row(precursor, "ATTRIBUTE", None, "spectrumRef", f"scan={position}"),
            row(f"{precursor}/isolationWindow", "CV", "MS:1000827", "isolation window target m/z", mz, mz_unit),
            row(f"{precursor}/selectedIonList/selectedIon[0]", "CV", "MS:1000744", "selected ion m/z", mz, mz_unit),
            row(f"{precursor}/selectedIonList/selectedIon[1]", "CV", "MS:1000041", "charge state", "2"),
            row(f"{precursor}/activation", "CV", "MS:1000133", "collision-induced dissociation", None),
        ]
    for number, (accession, name) in enumerate([("MS:1000514", "m/z array"), ("MS:1000515", "intensity array")]):
        array = f"spectrum/binaryDataArrayList/binaryDataArray[{number}]"
        rows += [
            row(array, "ATTRIBUTE", None, "encodedLength", "24"),
            row(array, "CV", "MS:1000523", "64-bit float", None),
            row(array, "CV", "MS:1000576", "no compression", None),
            row(array, "CV", accession, name, None),
        ]
    return rows


def get_library_id(rows):
    return next(row["value"] for row in rows if row["name"] == "library_id")


def is_inside(read, ranges):
    offset, length = read
    return any(start <= offset and offset + length <= start + size for start, size in ranges)


EVERY_SCAN = {"from": 0.0, "to": 100.0}


@unittest.skipIf(support.library_problem, support.library_problem)
class ScanMetadata(unittest.TestCase):
    def setUp(self):
        self.mzml = support.quantion.parse_mzml(write_mzml())
        self.addCleanup(self.mzml.dispose)
        self.bytes = support.quantion.write_ion(self.mzml)
        self.ion = support.quantion.parse_ion(self.bytes)
        self.addCleanup(self.ion.dispose)

    def test_scan_index_is_the_position_in_the_spectrum_list(self):
        for file in (self.mzml, self.ion):
            scans = support.quantion.get_scans(file, rt=EVERY_SCAN, options={"level": 0})
            self.assertEqual([scan["index"] for scan in scans], [0, 1, 2, 3, 4, 5])
            scans = support.quantion.get_scans(file, rt=EVERY_SCAN, options={"level": 2})
            self.assertEqual([scan["index"] for scan in scans], [1, 3, 5])

    def test_scan_index_is_set_when_reading_chosen_arrays(self):
        arrays = [{"key": "intensity", "accession": "MS:1000515"}]
        for file in (self.mzml, self.ion):
            scans = support.quantion.get_scans(file, rt=EVERY_SCAN, options={"level": 2, "arrays": arrays})
            self.assertEqual([scan["index"] for scan in scans], [1, 3, 5])

    def test_index_is_a_whole_number(self):
        scan = support.quantion.get_scans(self.ion, mz=303.0, options={"level": 2})[0]
        self.assertIs(type(scan["index"]), int)
        self.assertEqual(scan["index"], 3)

    def test_metadata_has_every_tag_in_tree_order(self):
        for file in (self.mzml, self.ion):
            for position in range(SCAN_COUNT):
                self.assertEqual(support.quantion.get_scan_metadata(file, position), expected_rows(position))

    def test_metadata_is_the_same_from_mzml_and_ion(self):
        for position in range(SCAN_COUNT):
            self.assertEqual(
                support.quantion.get_scan_metadata(self.mzml, position),
                support.quantion.get_scan_metadata(self.ion, position),
            )

    def test_metadata_of_found_scans_matches_by_index(self):
        for scan in support.quantion.get_scans(self.ion, rt=EVERY_SCAN, options={"level": 2}):
            rows = support.quantion.get_scan_metadata(self.ion, scan["index"])
            self.assertEqual(get_library_id(rows), f"SYNTHETIC{scan['index']:03}")

    def test_kind_is_a_scan_param_kind(self):
        kinds = {row["kind"] for row in support.quantion.get_scan_metadata(self.ion, 1)}
        self.assertEqual(kinds, set(support.quantion.ScanParamKind))
        for kind in kinds:
            self.assertIsInstance(kind, support.quantion.ScanParamKind)

    def test_missing_fields_are_none(self):
        rows = support.quantion.get_scan_metadata(self.ion, 0)
        centroid = next(row for row in rows if row["name"] == "centroid spectrum")
        self.assertIsNone(centroid["value"])
        self.assertIsNone(centroid["unit"])
        self.assertEqual(centroid["accession"], "MS:1000127")

    def test_tags_keep_only_the_chosen_tags(self):
        for file in (self.mzml, self.ion):
            rows = support.quantion.get_scan_metadata(file, 1, {"tags": ["selectedIon"]})
            self.assertEqual([row["name"] for row in rows], ["selected ion m/z", "charge state"])
            rows = support.quantion.get_scan_metadata(file, 1, {"tags": ("activation", "scan")})
            self.assertEqual(
                [row["path"] for row in rows],
                ["spectrum/scanList/scan[0]"] * 3 + ["spectrum/precursorList/precursor[0]/activation"],
            )

    def test_empty_tags_keep_every_tag(self):
        rows = support.quantion.get_scan_metadata(self.ion, 1, {"tags": []})
        self.assertEqual(rows, expected_rows(1))

    def test_bad_tags_are_refused(self):
        for tags in (["cvParam"], ["Scan"], ["scan[0]"], [""], [1], "scan", None):
            with self.assertRaises(support.quantion.QuantionError) as caught:
                support.quantion.get_scan_metadata(self.ion, 0, {"tags": tags})
            self.assertEqual(caught.exception.kind, "invalid_argument")
            self.assertEqual(caught.exception.argument, "tags")

    def test_unknown_option_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.get_scan_metadata(self.ion, 0, {"tag": ["scan"]})
        self.assertEqual(caught.exception.argument, "tag")

    def test_index_out_of_range_is_refused(self):
        for file in (self.mzml, self.ion):
            with self.assertRaises(support.quantion.QuantionError) as caught:
                support.quantion.get_scan_metadata(file, SCAN_COUNT)
            self.assertEqual(caught.exception.kind, "invalid_argument")
            self.assertEqual(caught.exception.argument, "index")

    def test_bad_index_is_refused(self):
        for index in (-1, 1.5, "1", True, None, float("nan")):
            with self.assertRaises(support.quantion.QuantionError) as caught:
                support.quantion.get_scan_metadata(self.ion, index)
            self.assertEqual(caught.exception.kind, "invalid_argument")
            self.assertEqual(caught.exception.argument, "index")

    def test_index_cannot_be_an_array_key(self):
        arrays = [{"key": "index", "accession": "MS:1000786"}]
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.get_scans(self.ion, rt=EVERY_SCAN, options={"arrays": arrays})
        self.assertEqual(caught.exception.argument, "arrays.key")


@unittest.skipIf(support.library_problem, support.library_problem)
class RemoteScanMetadata(unittest.TestCase):
    def setUp(self):
        mzml = support.quantion.parse_mzml(write_mzml())
        self.addCleanup(mzml.dispose)
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        path = Path(folder.name) / "library.ion"
        path.write_bytes(support.quantion.write_ion(mzml))
        self.server = support.RangeServer(path)
        self.addCleanup(self.server.stop)

    def open_with_only_planned_bytes(self):
        file = support.quantion.parse_ion(self.server.url)
        self.addCleanup(file.dispose)
        source = file._remote_source
        opened = [(0, source.HEADER_BYTES)] + support.api._plan_open(source.cache[(0, source.HEADER_BYTES)])
        planned = []
        reads = []
        keep_prefetch = source.prefetch
        keep_read = source.read

        def prefetch(ranges):
            planned.extend(ranges)
            keep_prefetch(ranges)

        def read(offset, length):
            reads.append((offset, length))
            return keep_read(offset, length)

        source.prefetch = prefetch
        source.read = read
        return file, opened, planned, reads

    def test_metadata_reads_only_planned_bytes(self):
        for position in (0, 3, SCAN_COUNT - 1):
            file, opened, planned, reads = self.open_with_only_planned_bytes()
            self.assertEqual(support.quantion.get_scan_metadata(file, position), expected_rows(position))
            self.assertGreater(len(planned), 0)
            for read in reads:
                self.assertTrue(is_inside(read, opened + planned), f"{read} was not planned")

    def test_index_out_of_range_is_refused_before_any_read(self):
        file, _, planned, reads = self.open_with_only_planned_bytes()
        with self.assertRaises(support.quantion.QuantionError) as caught:
            support.quantion.get_scan_metadata(file, SCAN_COUNT)
        self.assertEqual(caught.exception.argument, "index")
        self.assertEqual(planned, [])
        self.assertEqual(reads, [])


if __name__ == "__main__":
    unittest.main()
