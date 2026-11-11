import unittest

from tests import support

FOLDER = support.REPO_ROOT / "core" / "tests" / "fixtures" / "annotate"
SAMPLES = ["sample_peak_and_ms2", "sample_peak_only", "sample_no_peak"]
FEATURES = [{"mz": 195.0877, "rt": 2.0}]
WITHOUT_CANDIDATES = [{"mz": 350.05, "rt": 2.0}, {"mz": 500.0, "rt": 2.0}]


def get_plain(value):
    if isinstance(value, dict):
        return {key: get_plain(item) for key, item in value.items()}
    if isinstance(value, list):
        return [get_plain(item) for item in value]
    return value.tolist() if hasattr(value, "tolist") else value


def get_options(model="model", **changes):
    return {"library": str(FOLDER / "library.ion"), "model": str(FOLDER / f"{model}.onnx")} | changes


@unittest.skipIf(support.library_problem, support.library_problem)
class AnnotateFeatures(unittest.TestCase):
    def setUp(self):
        self.files = [self.open(str(FOLDER / f"{name}.ion")) for name in SAMPLES]

    def open(self, source):
        file = support.quantion.parse_ion(source)
        self.addCleanup(file.dispose)
        return file

    def annotate(self, options, features=FEATURES, files=None):
        return support.quantion.annotate_features(self.files if files is None else files, features, options)

    def test_the_best_compound_names_the_feature(self):
        first = self.annotate(get_options())[0]
        self.assertEqual((first["label"], first["id"], first["database"]), ("Caffeine", "ENTRY_1", "SOURCE_A"))
        self.assertAlmostEqual(first["frequency"], 1 / 3)
        self.assertGreater(first["score"], 0.99)
        self.assertGreater(first["cosine"], 0.99)

    def test_each_file_keeps_its_best_matches_in_order(self):
        files = self.annotate(get_options())[0]["files"]
        self.assertEqual([file["file"] for file in files], [0, 1, 2])
        self.assertIsNotNone(files[0]["scan_index"])
        self.assertIsNone(files[1]["scan_index"])
        matches = files[0]["matches"]
        self.assertEqual([found["label"] for found in matches], ["Caffeine", "Compound Y"])
        self.assertGreaterEqual(matches[0]["score"], matches[1]["score"])
        self.assertEqual(matches[0]["mz"].tolist(), [110.0713, 123.0427, 138.0662])
        self.assertEqual(matches[0]["metadata"], {"family": "FAMILY_C", "note": "synthetic 1"})

    def test_matches_limits_the_list_of_each_file(self):
        matches = self.annotate(get_options(matches=1))[0]["files"][0]["matches"]
        self.assertEqual([found["label"] for found in matches], ["Caffeine"])

    def test_a_feature_without_candidates_has_no_label(self):
        for annotation in self.annotate(get_options(), WITHOUT_CANDIDATES):
            self.assertIsNone(annotation["label"])
            self.assertIsNone(annotation["score"])
            self.assertEqual(annotation["frequency"], 0.0)
            self.assertTrue(all(not file["matches"] for file in annotation["files"]))

    def test_find_features_output_is_accepted(self):
        features = [FEATURES[0] | {"from": 1.9, "to": 2.1, "intensity": 1.0}]
        self.assertEqual(self.annotate(get_options(), features)[0]["label"], "Caffeine")

    def test_no_features_give_no_annotations(self):
        self.assertEqual(self.annotate(get_options(), []), [])

    def test_no_files_are_refused(self):
        with self.assertRaises(support.quantion.QuantionError) as caught:
            self.annotate(get_options(), files=[])
        self.assertEqual(caught.exception.argument, "files")

    def test_invalid_options_are_refused(self):
        cases = {"matches": 0, "rt_range": 0.0, "min_relative_intensity": 2.0, "library": ""}
        for key, value in cases.items():
            with self.subTest(key=key), self.assertRaises(support.quantion.QuantionError) as caught:
                self.annotate(get_options(**{key: value}))
            self.assertEqual(caught.exception.argument, key)

    def test_unknown_options_are_refused(self):
        with self.assertRaises(support.quantion.QuantionError):
            self.annotate(get_options(color="red"))

    def test_models_with_missing_or_invalid_keys_are_refused(self):
        for model, key in {"model_without_label": "label", "model_with_bad_max_peaks": "max_peaks"}.items():
            with self.subTest(model=model), self.assertRaisesRegex(support.quantion.QuantionError, key):
                self.annotate(get_options(model))

    def test_urls_give_the_same_result_with_few_requests(self):
        local = self.annotate(get_options())
        servers = {name: support.RangeServer(FOLDER / f"{name}") for name in ["library.ion", "model.onnx", "sample_peak_and_ms2.ion"]}
        for server in servers.values():
            self.addCleanup(server.stop)
        sample = self.open(servers["sample_peak_and_ms2.ion"].url)
        for server in servers.values():
            server.reset()
        options = {"library": servers["library.ion"].url, "model": servers["model.onnx"].url}
        remote = self.annotate(options, files=[sample, self.files[1], self.files[2]])
        self.assertEqual(get_plain(remote), get_plain(local))
        requests = {name: server.requests for name, server in servers.items()}
        self.assertEqual(requests, {"library.ion": 4, "model.onnx": 1, "sample_peak_and_ms2.ion": 1})


if __name__ == "__main__":
    unittest.main()
