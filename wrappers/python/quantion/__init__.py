from __future__ import annotations

from quantion._bridge import _ABI, load_library
from quantion import api as _api
from quantion.api import (
    parse_ion,
    parse_mzml,
    write_ion,
    write_mzml,
    get_scans,
    get_scan_metadata,
    calculate_eic,
    get_ion_image,
    find_peaks,
    get_peak,
    fit_peak,
    draw_peaks,
    find_noise_level,
    calculate_baseline,
    find_targets,
    find_transitions,
    find_features,
    deisotope_features,
    match_features,
    load_model,
    load_warp,
)
from quantion.file import SampleFile
from quantion.model import Model
from quantion.warp_model import WarpModel
from quantion._errors import QuantionError
from quantion._types import (
    TimeUnit,
    ScanParamKind,
    PeakShape,
    FromTo,
    Target,
    Transition,
    Header,
    FindPeaksOptions,
    GetPeakOptions,
    FitPeakOptions,
    DrawPeaksOptions,
    CalculateBaselineOptions,
    CalculateEicOptions,
    GridOptions,
    GroupingOptions,
    FindTargetsOptions,
    FindTransitionsOptions,
    FindFeaturesOptions,
    DeisotopeFeaturesOptions,
    MatchFeaturesOptions,
    GetScanMetadataOptions,
    GetScansOptions,
    ScanArray,
    ParseIonOptions,
    WriteIonOptions,
    WriteMzmlOptions,
    GetIonImageOptions,
    Peak,
    Feature,
    DataXY,
    NoiseLevel,
    PeakParameters,
    PredictedRt,
    Scan,
    ScanParam,
    IonImage,
)

__version__ = "0.2.1"

__all__ = [
    "Quantion",
    "QuantionError",
    "SampleFile",
    "Model",
    "WarpModel",
    "parse_ion",
    "parse_mzml",
    "write_ion",
    "write_mzml",
    "get_scans",
    "get_scan_metadata",
    "calculate_eic",
    "get_ion_image",
    "find_peaks",
    "get_peak",
    "fit_peak",
    "draw_peaks",
    "find_noise_level",
    "calculate_baseline",
    "find_targets",
    "find_transitions",
    "find_features",
    "deisotope_features",
    "match_features",
    "load_model",
    "load_warp",
    "TimeUnit",
    "ScanParamKind",
    "PeakShape",
    "FromTo",
    "Target",
    "Transition",
    "Header",
    "FindPeaksOptions",
    "GetPeakOptions",
    "FitPeakOptions",
    "DrawPeaksOptions",
    "CalculateBaselineOptions",
    "CalculateEicOptions",
    "GridOptions",
    "GroupingOptions",
    "FindTargetsOptions",
    "FindTransitionsOptions",
    "FindFeaturesOptions",
    "DeisotopeFeaturesOptions",
    "MatchFeaturesOptions",
    "GetScanMetadataOptions",
    "GetScansOptions",
    "ScanArray",
    "ParseIonOptions",
    "WriteIonOptions",
    "WriteMzmlOptions",
    "GetIonImageOptions",
    "Peak",
    "Feature",
    "DataXY",
    "NoiseLevel",
    "PeakParameters",
    "PredictedRt",
    "Scan",
    "ScanParam",
    "IonImage",
]


class Quantion:
    def __init__(self, abi: _ABI) -> None:
        self._abi = abi
        _api._set_abi(abi)

    def parse_ion(self, source, options=None) -> SampleFile:
        return parse_ion(source, options)

    def parse_mzml(self, source) -> SampleFile:
        return parse_mzml(source)

    def write_ion(self, file: SampleFile, options=None):
        return write_ion(file, options)

    def write_mzml(self, file: SampleFile, options=None):
        return write_mzml(file, options)

    def get_scans(self, file: SampleFile, *, rt=None, mz=None, options=None):
        return get_scans(file, rt=rt, mz=mz, options=options)

    def get_scan_metadata(self, file: SampleFile, index: int, options=None):
        return get_scan_metadata(file, index, options)

    def calculate_eic(self, file: SampleFile, mz: float, options=None):
        return calculate_eic(file, mz, options)

    def get_ion_image(self, file: SampleFile, mz: float, options=None):
        return get_ion_image(file, mz, options)

    def find_peaks(self, x, y, options=None):
        return find_peaks(x, y, options)

    def get_peak(self, x, y, rt: float, options=None):
        return get_peak(x, y, rt, options)

    def fit_peak(self, x, y, rt: float, options=None):
        return fit_peak(x, y, rt, options)

    def draw_peaks(self, params, options=None):
        return draw_peaks(params, options)

    def find_noise_level(self, y):
        return find_noise_level(y)

    def calculate_baseline(self, y, options=None):
        return calculate_baseline(y, options)

    def find_targets(self, file: SampleFile, targets, options=None):
        return find_targets(file, targets, options)

    def find_transitions(self, file: SampleFile, targets, options=None):
        return find_transitions(file, targets, options)

    def find_features(self, file: SampleFile, options=None):
        return find_features(file, options)

    def match_features(self, directory, options=None):
        return match_features(directory, options)

    def deisotope_features(self, features, options=None):
        return deisotope_features(features, options)

    def load_model(self, source) -> Model:
        return load_model(source)

    def load_warp(self, model: Model) -> WarpModel:
        return load_warp(model)

    def __repr__(self) -> str:
        return "<Quantion ready>"


_, _abi = load_library()
_api._set_abi(_abi)
