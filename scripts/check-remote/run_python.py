import json
import os
import struct
import sys
from pathlib import Path
from urllib.request import urlopen

MASS = 90.05550
FROM_RT = 0.0
TO_RT = 5.0
EIC_PPM = 20.0
EIC_MZ = 0.005
EIC = {"from": FROM_RT, "to": TO_RT, "ppm_tolerance": EIC_PPM, "mz_tolerance": EIC_MZ}
WINDOW = {"from": FROM_RT, "to": TO_RT}


def same_bits(left, right) -> bool:
    return len(left) == len(right) and all(
        struct.pack(">d", a) == struct.pack(">d", b) for a, b in zip(left, right)
    )


def stats(url: str, path: str) -> dict:
    origin = "/".join(url.split("/")[:3])
    with urlopen(origin + path) as response:
        return json.loads(response.read())


def main() -> int:
    url = os.environ.get("QUANTION_REMOTE_URL", "")
    if not url:
        print("QUANTION_REMOTE_URL is not set", file=sys.stderr)
        return 1

    root = Path(__file__).resolve().parents[2]
    fixture = root / "core/tests/fixtures/api/api.ion"
    sys.path.insert(0, str(root / "wrappers/python"))
    import quantion

    stats(url, "/reset")
    remote = quantion.parse_ion(url)
    opening = stats(url, "/stats")

    stats(url, "/reset")
    eic_remote = quantion.calculate_eic(remote, MASS, EIC)
    one_query = stats(url, "/stats")

    local = quantion.parse_ion(str(fixture))
    eic_local = quantion.calculate_eic(local, MASS, EIC)

    same = same_bits(eic_remote["x"], eic_local["x"]) and same_bits(
        eic_remote["y"], eic_local["y"]
    )

    print(f"file_bytes = {fixture.stat().st_size}")
    print(f"opening_bytes = {opening['bytes_sent']}")
    print(f"opening_requests = {opening['requests']}")
    print(f"query_bytes = {one_query['bytes_sent']}")
    print(f"query_requests = {one_query['requests']}")
    print(f"matches_local = {'yes' if same else 'no'}")

    model = str(root / "core/tests/fixtures/shift_model.onnx")
    warp = quantion.load_warp(quantion.load_model(model))
    target = {"id": "a", "mz": 89.04768, "rt": 2.385, "range": 0.1}
    calls = {
        "calculate_eic": lambda file: quantion.calculate_eic(file, MASS, WINDOW),
        "get_scans": lambda file: quantion.get_scans(file, rt={"from": 2.3, "to": 2.5}),
        "find_targets": lambda file: quantion.find_targets(file, [target], {"eic": WINDOW}),
        "find_transitions": lambda file: quantion.find_transitions(file, [{"id": "a", "precursor": 89.04768, "product": 89.04768, "rt": 2.385, "range": 0.1}]),
        "find_features": lambda file: quantion.find_features(file, {"eic": WINDOW}),
        "write_mzml": lambda file: quantion.write_mzml(file),
        "write_ion": lambda file: quantion.write_ion(file),
        "predict_rts": lambda file: warp.predict_rts(file, [{"mz": 250.1, "rt": 1.2}]),
    }
    for name, call in calls.items():
        print(f"fresh_{name} = {compare(call, url, str(fixture), quantion)}")
    print(f"upload = {check_upload(url, local, eic_local, quantion)}")
    return 0


def check_upload(url: str, local, eic_local, quantion) -> str:
    target = url.rsplit("/", 1)[0] + "/uploaded.ion"
    try:
        quantion.write_ion(local, {"out": target, "headers": [{"name": "x-check", "value": "upload"}]})
        uploaded = quantion.calculate_eic(quantion.parse_ion(target), MASS, EIC)
    except quantion.QuantionError as error:
        return f"error: {error}"
    same = same_bits(uploaded["x"], eic_local["x"]) and same_bits(uploaded["y"], eic_local["y"])
    return "same" if same else "different"


def to_text(value) -> str:
    return json.dumps(value, sort_keys=True, default=lambda item: item.tolist() if hasattr(item, "tolist") else str(item))


def compare(call, url: str, fixture: str, quantion) -> str:
    try:
        remote = call(quantion.parse_ion(url))
    except quantion.QuantionError as error:
        return "refused" if error.kind == "remote_file" else "error " + str(error)[:80]
    local = call(quantion.parse_ion(fixture))
    return "same" if to_text(remote) == to_text(local) else "differs"


if __name__ == "__main__":
    sys.exit(main())
