import base64
import struct
import unittest

import numpy as np

from tests import support

SCAN_COUNT = 20
PEAK_COUNT = 50
MEGABYTE = 1024 * 1024
BLOCK_SIZE_OFFSET = 16
MZ_WINDOW_OFFSET = 24
EVERY_SCAN = {"from": 0.0, "to": 100.0}


def encode(values):
    return base64.b64encode(np.asarray(values, dtype=np.float64).tobytes()).decode()


def write_array(accession, name, values):
    encoded = encode(values)
    return (
        f'<binaryDataArray encodedLength="{len(encoded)}">'
        '<cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/>'
        '<cvParam cvRef="MS" accession="MS:1000576" name="no compression"/>'
        f'<cvParam cvRef="MS" accession="{accession}" name="{name}"/>'
        f"<binary>{encoded}</binary></binaryDataArray>"
    )


def write_spectrum(position):
    mz = 100.0 + 18.0 * np.arange(PEAK_COUNT) + position * 0.01
    intensity = 1000.0 + 10.0 * np.arange(PEAK_COUNT) + position
    return (
        f'<spectrum index="{position}" id="scan={position + 1}" defaultArrayLength="{PEAK_COUNT}">'
        '<cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/>'
        '<scanList count="1"><scan>'
        f'<cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{position * 0.1}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/>'
        "</scan></scanList>"
        '<binaryDataArrayList count="2">'
        + write_array("MS:1000514", "m/z array", mz)
        + write_array("MS:1000515", "intensity array", intensity)
        + "</binaryDataArrayList></spectrum>"
    )


def write_mzml():
    spectra = "".join(write_spectrum(position) for position in range(SCAN_COUNT))
    return (
        '<?xml version="1.0"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0">'
        '<cvList count="1"><cv id="MS" fullName="x" URI="x"/></cvList><run id="r">'
        f'<spectrumList count="{SCAN_COUNT}" defaultDataProcessingRef="d">{spectra}</spectrumList></run></mzML>'
    ).encode()


def read_header(ion):
    block_bytes = struct.unpack_from("<Q", ion, BLOCK_SIZE_OFFSET)[0]
    mz_window = struct.unpack_from("<I", ion, MZ_WINDOW_OFFSET)[0]
    return block_bytes, mz_window


def get_peaks(file):
    scans = support.quantion.get_scans(file, rt=EVERY_SCAN, options={"level": 0})
    return [(scan["mz"].tolist(), scan["intensity"].tolist()) for scan in scans]


@unittest.skipIf(support.library_problem, support.library_problem)
class WriteIonBlocks(unittest.TestCase):
    def setUp(self):
        self.mzml = support.quantion.parse_mzml(write_mzml())
        self.addCleanup(self.mzml.dispose)

    def write(self, options=None):
        return support.quantion.write_ion(self.mzml, options)

    def test_defaults_are_8_mb_blocks_and_20_mz_windows(self):
        self.assertEqual(read_header(self.write()), (8 * MEGABYTE, 20))

    def test_chosen_values_are_recorded_and_peaks_read_back_the_same(self):
        expected = get_peaks(self.mzml)
        self.assertEqual(len(expected), SCAN_COUNT)
        for block_size, mz_window in ((1, 20), (2, 5), (16, 0), (8, 100)):
            ion = self.write({"block_size": block_size, "mz_window": mz_window})
            self.assertEqual(read_header(ion), (block_size * MEGABYTE, mz_window))
            reopened = support.quantion.parse_ion(ion)
            self.addCleanup(reopened.dispose)
            self.assertEqual(get_peaks(reopened), expected)

    def test_bad_block_size_is_refused(self):
        for block_size in (0, -1, 1.5, 2**32, None, True, float("nan"), [8]):
            with self.assertRaises(support.quantion.QuantionError) as caught:
                self.write({"block_size": block_size})
            self.assertEqual(caught.exception.kind, "invalid_argument")
            self.assertEqual(caught.exception.argument, "block_size")

    def test_bad_mz_window_is_refused(self):
        for mz_window in (-1, 2.5, 2**32, None, True, float("inf"), [20]):
            with self.assertRaises(support.quantion.QuantionError) as caught:
                self.write({"mz_window": mz_window})
            self.assertEqual(caught.exception.kind, "invalid_argument")
            self.assertEqual(caught.exception.argument, "mz_window")

    def test_unknown_option_is_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            self.write({"blocksize": 8})
        self.assertEqual(caught.exception.kind, "unknown_option")


if __name__ == "__main__":
    unittest.main()
