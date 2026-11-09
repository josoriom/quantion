import os
import struct
import sys

import quantion

FROM_RT = 0.0
TO_RT = 5.0
FROM_TO = {"from": FROM_RT, "to": TO_RT}
EIC_PPM = 20.0
EIC_MZ = 0.005
EIC = {"from": FROM_RT, "to": TO_RT, "ppm_tolerance": EIC_PPM, "mz_tolerance": EIC_MZ}
GRID_START = 89.0
GRID_END = 91.0
GRID_STEP = 0.005
ROI_RANGE = 0.05
THREADS = 1

FEATURE_MIN_INTENSITY = 500.0
FEATURE_MIN_PEAK_WIDTH_POINTS = 5

SELF_CHECK_VALUES = (1.0, 2.0, 3.0)

CHANNELS = [
    {
        "name": "mz_2",
        "mass": 90.05550,
        "min_intensity": 500.0,
        "min_peak_width_points": 5,
        "targets": [("met_1", 2.885), ("met_2", 2.552), ("met_3", 2.465)],
    },
    {
        "name": "mz_1",
        "mass": 89.04768,
        "min_intensity": 500.0,
        "min_peak_width_points": 5,
        "targets": [("met_1", 2.385), ("met_2", 2.18), ("met_3", 2.08)],
    },
]


def number(value):
    value = float(value)
    if value != value:
        return "nan"
    if value == float("inf"):
        return "inf"
    if value == float("-inf"):
        return "-inf"
    return repr(value)


def digest(values):
    hash_value = 0x811C9DC5
    for value in values:
        for byte in struct.pack("<d", float(value)):
            hash_value ^= byte
            hash_value = (hash_value * 0x01000193) & 0xFFFFFFFF
    return f"{hash_value:08x}"


def peak_options(channel):
    return {
        "min_intensity": channel["min_intensity"],
        "min_peak_width_points": channel["min_peak_width_points"],
        "auto_noise": True,
        "auto_baseline": True,
    }


def chrom_peak_options(channel):
    return {
        "min_intensity": channel["min_intensity"],
        "min_peak_width_points": channel["min_peak_width_points"],
        "auto_noise": False,
        "auto_baseline": False,
    }


def feature_peak_options():
    return {
        "min_intensity": FEATURE_MIN_INTENSITY,
        "min_peak_width_points": FEATURE_MIN_PEAK_WIDTH_POINTS,
        "auto_noise": True,
        "auto_baseline": True,
    }


def peak_lines(prefix, peak, out):
    shared_peak_lines(prefix, peak, out)
    out.append(f"{prefix}.n_points = {int(peak['n_points'])}")


def shared_peak_lines(prefix, peak, out):
    out.append(f"{prefix}.rt = {number(peak['rt'])}")
    out.append(f"{prefix}.from = {number(peak['from'])}")
    out.append(f"{prefix}.to = {number(peak['to'])}")
    out.append(f"{prefix}.integral = {number(peak['integral'])}")
    out.append(f"{prefix}.intensity = {number(peak['intensity'])}")


def report_channel(file, transitions, channel, out):
    name = channel["name"]
    eic = quantion.calculate_eic(file, channel["mass"], EIC)
    x, y = list(eic["x"]), list(eic["y"])

    out.append(f"{name}.calculate_eic.points = {len(x)}")
    out.append(f"{name}.calculate_eic.x.fnv1a = {digest(x)}")
    out.append(f"{name}.calculate_eic.y.fnv1a = {digest(y)}")

    noise = quantion.find_noise_level(y)
    out.append(f"{name}.find_noise_level.intensity = {number(noise['intensity'])}")
    out.append(f"{name}.find_noise_level.width = {noise['width']}")

    found = quantion.find_peaks(x, y, peak_options(channel))
    out.append(f"{name}.find_peaks.count = {len(found)}")
    for index, peak in enumerate(found):
        peak_lines(f"{name}.find_peaks[{index}]", peak, out)

    target = channel["targets"][2][1]
    single = quantion.get_peak(x, y, target, {"range": ROI_RANGE, "find_peaks": peak_options(channel)})
    peak_lines(f"{name}.get_peak", single, out)

    targets = [
        {"id": id_, "rt": rt, "mz": channel["mass"], "range": ROI_RANGE}
        for id_, rt in channel["targets"]
    ]
    rows = quantion.find_targets(
        file, targets, {"eic": EIC, "find_peaks": peak_options(channel), "threads": THREADS}
    )
    for target, row in zip(targets, rows):
        shared_peak_lines(f"{name}.find_targets.{target['id']}", row, out)

    chrom_targets = [
        {"id": id_, "precursor": channel["mass"], "product": channel["mass"], "rt": rt, "range": ROI_RANGE}
        for id_, rt in channel["targets"]
    ]
    chrom_rows = quantion.find_transitions(
        transitions, chrom_targets, {"find_peaks": chrom_peak_options(channel), "threads": THREADS}
    )
    for (id_, _), row in zip(channel["targets"], chrom_rows):
        prefix = f"{name}.find_transitions.{id_}"
        out.append(f"{prefix}.rt = {number(row['rt'])}")
        out.append(f"{prefix}.from = {number(row['from'])}")
        out.append(f"{prefix}.to = {number(row['to'])}")
        out.append(f"{prefix}.intensity = {number(row['intensity'])}")
        out.append(f"{prefix}.area = {number(row['integral'])}")


def main():
    directory = sys.argv[1] if len(sys.argv) > 1 else "core/tests/fixtures/api"
    fixture = os.path.join(directory, "api.ion")

    file = quantion.parse_ion(fixture)
    transitions = quantion.parse_ion(os.path.join(os.path.dirname(directory), "srm.ion"))
    out = []
    out.append(f"fnv1a.self_check = {digest(SELF_CHECK_VALUES)}")

    scans = quantion.get_scans(file, rt=FROM_TO)
    out.append(f"parse_ion.scans = {len(scans)}")
    out.append(f"get_scans.first.index = {scans[0]['index']}")
    out.append(f"get_scans.last.index = {scans[-1]['index']}")
    rows = quantion.get_scan_metadata(file, scans[-1]["index"])
    out.append(f"get_scan_metadata.count = {len(rows)}")
    for position, row in enumerate(rows):
        fields = [row["path"], row["kind"].value, row["accession"], row["name"], row["value"], row["unit"], row["unit_accession"]]
        out.append(f"get_scan_metadata.{position} = {' | '.join('NA' if field is None else field for field in fields)}")
    scan_rows = quantion.get_scan_metadata(file, scans[-1]["index"], {"tags": ["scan"]})
    out.append(f"get_scan_metadata.scan.count = {len(scan_rows)}")

    for channel in CHANNELS:
        report_channel(file, transitions, channel, out)

    feature_options = {
        "eic": EIC,
        "grid": {"start": GRID_START, "end": GRID_END, "step_size": GRID_STEP},
        "find_peaks": feature_peak_options(),
        "threads": THREADS,
    }

    found = quantion.find_features(file, feature_options)
    out.append(f"find_features.count = {len(found)}")
    for index, feature in enumerate(found):
        name = f"find_features[{index}]"
        out.append(f"{name}.mz = {number(feature['mz'])}")
        out.append(f"{name}.rt = {number(feature['rt'])}")
        out.append(f"{name}.from = {number(feature['from'])}")
        out.append(f"{name}.to = {number(feature['to'])}")
        out.append(f"{name}.integral = {number(feature['integral'])}")
        out.append(f"{name}.intensity = {number(feature['intensity'])}")
        out.append(f"{name}.n_points = {int(feature['n_points'])}")

    consensus = quantion.match_features(directory, feature_options)
    for label, features in (("match_features", consensus), ("deisotope_features", quantion.deisotope_features(consensus))):
        out.append(f"{label}.count = {len(features)}")
        for index, feature in enumerate(features):
            name = f"{label}[{index}]"
            out.append(f"{name}.mz = {number(feature['mz'])}")
            out.append(f"{name}.rt = {number(feature['rt'])}")
            out.append(f"{name}.from = {number(feature['from'])}")
            out.append(f"{name}.to = {number(feature['to'])}")
            out.append(f"{name}.integral = {number(feature['integral'])}")
            out.append(f"{name}.intensity = {number(feature['intensity'])}")

    warp = quantion.load_warp(quantion.load_model(os.path.join(directory, "..", "shift_model.onnx")))
    targets = [
        {"id": f"{channel['name']}.{name}", "mz": channel["mass"], "rt": rt}
        for channel in CHANNELS
        for name, rt in channel["targets"]
    ]
    predicted = warp.predict_rts(file, targets)
    out.append(f"predict_rts.count = {len(predicted)}")
    for target, prediction in zip(targets, predicted):
        out.append(f"predict_rts.{target['id']}.rt = {number(prediction['rt'])}")
        out.append(f"predict_rts.{target['id']}.uncertainty = {number(prediction['uncertainty'])}")

    folder = os.path.join(directory, "..", "annotate")
    samples = [quantion.parse_ion(os.path.join(folder, f"{name}.ion")) for name in ("sample_peak_and_ms2", "sample_peak_only", "sample_no_peak")]
    options = {"library": os.path.join(folder, "library.ion"), "model": os.path.join(folder, "model.onnx")}
    found = quantion.annotate_features(samples, [{"mz": 195.0877, "rt": 2.0}], options)
    out.append(f"annotate_features.count = {len(found)}")
    for index, annotation in enumerate(found):
        name = f"annotate_features[{index}]"
        for key in ("label", "id", "database"):
            out.append(f"{name}.{key} = {'NA' if annotation[key] is None else annotation[key]}")
        out.append(f"{name}.frequency = {number(annotation['frequency'])}")
        for key in ("score", "cosine"):
            out.append(f"{name}.{key} = {'NA' if annotation[key] is None else number(annotation[key])}")
        for file in annotation["files"]:
            scan = "NA" if file["scan_index"] is None else file["scan_index"]
            out.append(f"{name}.files[{file['file']}].scan_index = {scan}")
            for rank, match in enumerate(file["matches"]):
                prefix = f"{name}.files[{file['file']}].matches[{rank}]"
                for key in ("label", "id", "database"):
                    out.append(f"{prefix}.{key} = {match[key]}")
                for key in ("score", "cosine"):
                    out.append(f"{prefix}.{key} = {number(match[key])}")

    print("\n".join(out))


if __name__ == "__main__":
    main()
