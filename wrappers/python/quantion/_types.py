from __future__ import annotations

from enum import Enum
from typing import Callable, Dict, List, Optional, TypedDict, Union

import numpy as np


class TimeUnit(str, Enum):
    """TimeUnit · Enums

    Unit of the retention times you give to a function. Results are always in minutes.

    Fields:
        MINUTES: Minutes (default).
        SECONDS: Seconds.

    Documentation: https://phenological.github.io/quantion/?language=python#TimeUnit
    """
    MINUTES = "MINUTES"
    SECONDS = "SECONDS"

    def __str__(self) -> str:
        return self.value


class PeakShape(str, Enum):
    """PeakShape · Enums

    Curve used to fit a peak.

    Fields:
        GAUSSIAN: Symmetric Gaussian.
        EMG: Exponentially modified Gaussian: a Gaussian with a tail on the right (default).

    Documentation: https://phenological.github.io/quantion/?language=python#PeakShape
    """
    GAUSSIAN = "GAUSSIAN"
    EMG = "EMG"

    def __str__(self) -> str:
        return self.value


FromTo = TypedDict("FromTo", {"from": float, "to": float})
FromTo.__doc__ = """FromTo · Inputs

A range: retention-time windows and m/z ranges.

Fields:
    from: Start of the range.
    to: End of the range.

Example:
    window: FromTo = {"from": 1.0, "to": 2.5}

Documentation: https://phenological.github.io/quantion/?language=python#FromTo
"""


class _TargetFields(TypedDict):
    mz: float
    rt: float


class Target(_TargetFields, total=False):
    """Target · Inputs

    One compound. range limits the search in find_targets; missing or 0 searches the whole EIC. predict_rts and calibrate do not use range.

    Fields:
        id: Your name for it. Not used in the search. Optional.
        mz: m/z (Da).
        rt: Expected retention time.
        range: Half-width of the search window around rt. Missing or 0 = search the whole EIC. Optional.

    Example:
        target: Target = {"id": "compound_1", "mz": 260.103, "rt": 2.383, "range": 0.05}

    Documentation: https://phenological.github.io/quantion/?language=python#Target
    """
    id: str
    range: float


class _TransitionFields(TypedDict):
    precursor: float
    product: float
    rt: float
    range: float


class Transition(_TransitionFields, total=False):
    """Transition · Inputs

    One SRM transition.

    Fields:
        id: Your name for it. Optional.
        precursor: Precursor (Q1) m/z.
        product: Product (Q3) m/z.
        rt: Expected retention time.
        range: Half-width of the search window around rt.

    Example:
        transition: Transition = {"id": "compound_1", "precursor": 89.04768, "product": 89.04768, "rt": 2.385, "range": 0.1}

    Documentation: https://phenological.github.io/quantion/?language=python#Transition
    """
    id: str


class Header(TypedDict):
    """Header · Inputs

    One HTTP header sent with an upload in write_ion. Values are never printed. A line break, or Host, Content-Length or Transfer-Encoding, gives an invalid_argument error.

    Fields:
        name: Name of the header, such as Authorization.
        value: Value of the header.

    Example:
        header: Header = {"name": "Authorization", "value": "Bearer <token>"}

    Documentation: https://phenological.github.io/quantion/?language=python#Header
    """
    name: str
    value: str


FindPeaksOptions = TypedDict("FindPeaksOptions", {
    "min_integral": float,
    "min_intensity": float,
    "min_peak_width_points": int,
    "noise": float,
    "auto_noise": bool,
    "auto_baseline": bool,
    "lambda": float,
    "max_iterations": int,
    "allow_overlap": bool,
    "min_snr": float,
    "min_r2": float,
    "shape": Union[PeakShape, str],
    "kernel_size": int,
}, total=False)
FindPeaksOptions.__doc__ = """FindPeaksOptions · Options

Peak-picking settings. Every field is optional.

Fields:
    min_integral: Drop peaks with a smaller area. Off when not given. Default off.
    min_intensity: Drop peaks with a lower apex. Default 500.
    min_peak_width_points: Drop peaks with fewer points. Default 3.
    noise: Fixed noise level, used when auto_noise is false. Default off.
    auto_noise: Estimate the noise from the trace. Default True.
    auto_baseline: Subtract an airPLS baseline first. Default True.
    lambda: airPLS smoothness. Default 20.
    max_iterations: Maximum airPLS iterations. Default 4.
    allow_overlap: Let neighbouring peaks share points. Default False.
    min_snr: Minimum signal-to-noise ratio. Default 2.
    min_r2: Minimum fit quality of the peak shape (0–1). 0 keeps every peak. Default 0.
    shape: Curve used for the fit. Default EMG.
    kernel_size: Smoothing window in points. 0 = no smoothing. Default 0.

Documentation: https://phenological.github.io/quantion/?language=python#FindPeaksOptions
"""


class GetPeakOptions(TypedDict, total=False):
    """GetPeakOptions · Options

    Settings for get_peak.

    Fields:
        range: Half-width of the search window around rt. 0 searches the whole trace. Default 0.
        find_peaks: Settings for peak picking. See FindPeaksOptions: https://phenological.github.io/quantion/?language=python#FindPeaksOptions

    Documentation: https://phenological.github.io/quantion/?language=python#GetPeakOptions
    """
    range: float
    find_peaks: FindPeaksOptions


class FitPeakOptions(TypedDict, total=False):
    """FitPeakOptions · Options

    Settings for fit_peak.

    Fields:
        shape: Curve to fit. Default EMG.
        intensity: Apex intensity to start the fit from. 0 = take it from the peak at rt. Default 0.

    Documentation: https://phenological.github.io/quantion/?language=python#FitPeakOptions
    """
    shape: Union[PeakShape, str]
    intensity: float


DrawPeaksOptions = TypedDict("DrawPeaksOptions", {
    "from": float,
    "to": float,
    "points": int,
}, total=False)
DrawPeaksOptions.__doc__ = """DrawPeaksOptions · Options

Settings for draw_peaks.

Fields:
    from: Start of the chromatogram. With to, 0 and 0 = fit the range around all the peaks. Default 0.
    to: End of the chromatogram. Default 0.
    points: Number of points drawn between from and to. Default 1000.

Documentation: https://phenological.github.io/quantion/?language=python#DrawPeaksOptions
"""


CalculateBaselineOptions = TypedDict("CalculateBaselineOptions", {
    "lambda": float,
    "max_iterations": int,
}, total=False)
CalculateBaselineOptions.__doc__ = """CalculateBaselineOptions · Options

airPLS baseline settings for calculate_baseline.

Fields:
    lambda: Smoothness. Larger = smoother. Default 20.
    max_iterations: Maximum iterations. Default 4.

Documentation: https://phenological.github.io/quantion/?language=python#CalculateBaselineOptions
"""


CalculateEicOptions = TypedDict("CalculateEicOptions", {
    "from": float,
    "to": float,
    "ppm_tolerance": float,
    "mz_tolerance": float,
    "time_unit": Union[TimeUnit, str],
}, total=False)
CalculateEicOptions.__doc__ = """CalculateEicOptions · Options

Which scans are read and how wide the m/z window is. Used by calculate_eic, and as the eic group of find_targets, find_features and match_features. The wider of the ppm and absolute m/z windows is used.

Fields:
    from: Start of the retention-time range read. Default 0.
    to: End of the retention-time range read. The default reads to the end of the run. Default inf.
    ppm_tolerance: Tolerance in ppm. Default 20.
    mz_tolerance: Absolute tolerance in Da. Default 0.005.
    time_unit: Unit of from, to and rt values given. Default MINUTES.

Documentation: https://phenological.github.io/quantion/?language=python#CalculateEicOptions
"""


class GridOptions(TypedDict, total=False):
    """GridOptions · Options

    The m/z grid that find_features and match_features search.

    Fields:
        start: Lowest m/z. Default 40.
        end: Highest m/z. Default 1000.
        step_size: Step between grid m/z values (Da). Default 0.005.

    Documentation: https://phenological.github.io/quantion/?language=python#GridOptions
    """
    start: float
    end: float
    step_size: float


class GroupingOptions(TypedDict, total=False):
    """GroupingOptions · Options

    How match_features matches features across samples.

    Fields:
        ppm_tolerance: Same feature when m/z is within this ppm…. Default 5.
        mz_tolerance: …or within this absolute m/z (Da). Default 0.0025.
        rt_tolerance: …and rt is within this many minutes. Default 0.05.
        min_frequency: Keep a feature only if found in at least this fraction of samples (0–1). Default 0.

    Documentation: https://phenological.github.io/quantion/?language=python#GroupingOptions
    """
    ppm_tolerance: float
    mz_tolerance: float
    rt_tolerance: float
    min_frequency: float


class FindTargetsOptions(TypedDict, total=False):
    """FindTargetsOptions · Options

    Settings for find_targets.

    Fields:
        eic: EIC retention-time range (from, to), tolerances and time unit. See CalculateEicOptions: https://phenological.github.io/quantion/?language=python#CalculateEicOptions
        find_peaks: Settings for peak picking. See FindPeaksOptions: https://phenological.github.io/quantion/?language=python#FindPeaksOptions
        threads: Number of threads. Default 1.

    Documentation: https://phenological.github.io/quantion/?language=python#FindTargetsOptions
    """
    eic: CalculateEicOptions
    find_peaks: FindPeaksOptions
    threads: int


class FindTransitionsOptions(TypedDict, total=False):
    """FindTransitionsOptions · Options

    Settings for find_transitions.

    Fields:
        find_peaks: Settings for peak picking. See FindPeaksOptions: https://phenological.github.io/quantion/?language=python#FindPeaksOptions
        threads: Number of threads. Default 1.

    Documentation: https://phenological.github.io/quantion/?language=python#FindTransitionsOptions
    """
    find_peaks: FindPeaksOptions
    threads: int


class FindFeaturesOptions(TypedDict, total=False):
    """FindFeaturesOptions · Options

    Settings for find_features.

    Fields:
        eic: EIC retention-time range (from, to), tolerances and time unit. See CalculateEicOptions: https://phenological.github.io/quantion/?language=python#CalculateEicOptions
        grid: The m/z grid. See GridOptions: https://phenological.github.io/quantion/?language=python#GridOptions
        find_peaks: Settings for peak picking. See FindPeaksOptions: https://phenological.github.io/quantion/?language=python#FindPeaksOptions
        threads: Number of threads. Default 1.

    Documentation: https://phenological.github.io/quantion/?language=python#FindFeaturesOptions
    """
    eic: CalculateEicOptions
    grid: GridOptions
    find_peaks: FindPeaksOptions
    threads: int


class DeisotopeFeaturesOptions(TypedDict, total=False):
    """DeisotopeFeaturesOptions · Options

    How deisotope_features links isotope peaks.

    Fields:
        rt_tolerance: Same compound when rt is within this many minutes of M. Default 0.02.
        mz_tolerance: Isotope peak when m/z is within this many Da of the expected position. Default 0.005.

    Documentation: https://phenological.github.io/quantion/?language=python#DeisotopeFeaturesOptions
    """
    rt_tolerance: float
    mz_tolerance: float


class MatchFeaturesOptions(TypedDict, total=False):
    """MatchFeaturesOptions · Options

    Settings for match_features: the find_features settings plus grouping.

    Fields:
        eic: EIC retention-time range (from, to), tolerances and time unit. See CalculateEicOptions: https://phenological.github.io/quantion/?language=python#CalculateEicOptions
        grid: The m/z grid. See GridOptions: https://phenological.github.io/quantion/?language=python#GridOptions
        grouping: How features are matched across samples. See GroupingOptions: https://phenological.github.io/quantion/?language=python#GroupingOptions
        find_peaks: Settings for peak picking. See FindPeaksOptions: https://phenological.github.io/quantion/?language=python#FindPeaksOptions
        threads: Number of threads. Default 1.

    Documentation: https://phenological.github.io/quantion/?language=python#MatchFeaturesOptions
    """
    eic: CalculateEicOptions
    grid: GridOptions
    grouping: GroupingOptions
    find_peaks: FindPeaksOptions
    threads: int


class ScanArray(TypedDict):
    """ScanArray · Options

    One array to read from each scan in get_scans.

    Fields:
        key: Name of the array in the result, as a field of the scan.
        accession: Accession of the array in the file, such as MS:1000514 for m/z.

    Example:
        array: ScanArray = {"key": "mz", "accession": "MS:1000514"}

    Documentation: https://phenological.github.io/quantion/?language=python#ScanArray
    """
    key: str
    accession: str


class GetScanMetadataOptions(TypedDict, total=False):
    """GetScanMetadataOptions · Options

    Settings for get_scan_metadata.

    Fields:
        tags: Which tags to keep, by name. A row is kept when the last tag of its path matches, so scan keeps spectrum/scanList/scan[0] but not its scanWindow. Empty keeps every tag. The names are: spectrum, referenceableParamGroupRef, spectrumDescription, scanList, scan, scanWindowList, scanWindow, precursorList, precursor, isolationWindow, selectedIonList, selectedIon, activation, productList, product, binaryDataArrayList, binaryDataArray. Default [].

    Documentation: https://phenological.github.io/quantion/?language=python#GetScanMetadataOptions
    """
    tags: List[str]


class GetScansOptions(TypedDict, total=False):
    """GetScansOptions · Options

    Settings for get_scans.

    Fields:
        level: MS level. 0 = every level. Default 1.
        time_unit: Unit of rt in the query. Default MINUTES.
        arrays: Which arrays to read from each scan, as keys with accessions. Every other field of Scan always comes back. Default {"key": "mz", "accession": "MS:1000514"}, {"key": "intensity", "accession": "MS:1000515"}. See ScanArray: https://phenological.github.io/quantion/?language=python#ScanArray

    Documentation: https://phenological.github.io/quantion/?language=python#GetScansOptions
    """
    level: int
    time_unit: Union[TimeUnit, str]
    arrays: List[ScanArray]


class AnnotateFeaturesOptions(TypedDict, total=False):
    """AnnotateFeaturesOptions · Options

    Settings for annotate_features.

    Fields:
        library: Spectral library: an Ionic (.ion) file path or URL. Each spectrum has a precursor m/z, a vector array and the params named by the model. Default "https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion".
        model: Spectrum model: an ONNX file path or URL. Default "https://github.com/josoriom/cipher/releases/latest/download/model.onnx".
        matches: How many matches to keep per file, best first. Default 3.
        ppm_tolerance: Precursor tolerance in ppm. The larger of the two tolerances is used. Default 20.
        mz_tolerance: Precursor tolerance in Da. Default 0.005.
        rt_range: Largest distance in minutes from the peak apex to the MS2 scan. Default 0.3.
        min_relative_intensity: MS2 peaks below this share of the base peak are dropped (0–1). Default 0.02.

    Documentation: https://phenological.github.io/quantion/?language=python#AnnotateFeaturesOptions
    """
    library: str
    model: str
    matches: int
    ppm_tolerance: float
    mz_tolerance: float
    rt_range: float
    min_relative_intensity: float


class ParseIonOptions(TypedDict, total=False):
    """ParseIonOptions · Options

    Settings for parse_ion.

    Fields:
        max_cache_size: Cache size in bytes. 0 = no limit. Default 0.

    Documentation: https://phenological.github.io/quantion/?language=python#ParseIonOptions
    """
    max_cache_size: int


class WriteIonOptions(TypedDict, total=False):
    """WriteIonOptions · Options

    Settings for write_ion.

    Fields:
        out: Path or URL to write to. Not given = return the bytes. Default None.
        overwrite: Replace an existing file. Default False.
        level: Compression level, 0 to 22. Default 12.
        f32_compress: Store intensities as 32-bit floats. Default False.
        block_size: Size of each data block before compression, in MB. A whole number, 1 or more. Default 8.
        mz_window: Width of the m/z windows the spectra are split into, in Da, so a query reads only the windows it needs. A whole number, 0 or more; 0 keeps each spectrum whole. Default 20.
        headers: Extra HTTP headers. Only with a URL. Default [].

    Documentation: https://phenological.github.io/quantion/?language=python#WriteIonOptions
    """
    out: str
    overwrite: bool
    level: int
    f32_compress: bool
    block_size: int
    mz_window: int
    headers: List[Header]


class WriteMzmlOptions(TypedDict, total=False):
    """WriteMzmlOptions · Options

    Settings for write_mzml.

    Fields:
        out: Path to write to. Not given = return the bytes. Default None.
        overwrite: Replace an existing file. Default False.

    Documentation: https://phenological.github.io/quantion/?language=python#WriteMzmlOptions
    """
    out: str
    overwrite: bool


class GetIonImageOptions(TypedDict, total=False):
    """GetIonImageOptions · Options

    Settings for get_ion_image.

    Fields:
        tolerance: Absolute m/z tolerance (Da). Default 0.125.
        level: MS level. Default 1.
        on_progress: Progress callback: fetched bytes, total bytes, bytes held. Default None.

    Documentation: https://phenological.github.io/quantion/?language=python#GetIonImageOptions
    """
    tolerance: float
    level: int
    on_progress: Callable[[int, int, int], None]


Peak = TypedDict("Peak", {
    "rt": float,
    "from": float,
    "to": float,
    "intensity": float,
    "integral": float,
    "n_points": int,
})
Peak.__doc__ = """Peak · Results

One chromatographic peak. No peak gives every field 0.

Fields:
    rt: Apex retention time (minutes).
    from: Start of the peak.
    to: End of the peak.
    intensity: Apex intensity.
    integral: Area of the peak.
    n_points: Points across the peak.

Documentation: https://phenological.github.io/quantion/?language=python#Peak
"""


_FeatureFields = TypedDict("_FeatureFields", {
    "mz": float,
    "rt": float,
    "from": float,
    "to": float,
    "intensity": float,
    "integral": float,
    "n_points": int,
})


class Feature(_FeatureFields, total=False):
    """Feature · Results

    A peak at an m/z. match_features adds frequency.

    Fields:
        mz: m/z (Da).
        rt: Apex retention time (minutes).
        from: Start of the peak.
        to: End of the peak.
        intensity: Apex intensity.
        integral: Area of the peak.
        n_points: Points across the peak.
        frequency: match_features only: fraction of samples where it was found (0–1). Optional.

    Documentation: https://phenological.github.io/quantion/?language=python#Feature
    """
    frequency: float


class DataXY(TypedDict):
    """DataXY · Results

    A trace: x = retention times, y = intensities. Returned by calculate_eic, taken by find_peaks.

    Fields:
        x: Retention times (minutes).
        y: Intensities.

    Documentation: https://phenological.github.io/quantion/?language=python#DataXY
    """
    x: np.ndarray
    y: np.ndarray


class NoiseLevel(TypedDict):
    """NoiseLevel · Results

    Result of find_noise_level.

    Fields:
        width: Window width (points) the estimate came from.
        intensity: Noise intensity.

    Documentation: https://phenological.github.io/quantion/?language=python#NoiseLevel
    """
    width: int
    intensity: float


class _PeakParametersFields(TypedDict):
    height: float
    center: float
    fwhm: float
    tail: float


class PeakParameters(_PeakParametersFields, total=False):
    """PeakParameters · Results

    One peak curve: returned by fit_peak, or written by hand to simulate. draw_peaks takes a list of them. tail = 0 is a Gaussian, tail above 0 an EMG.

    Fields:
        height: Height of the curve at its apex.
        center: Retention time of the apex (minutes).
        fwhm: Full width at half maximum of the Gaussian part (minutes). With a tail, the drawn peak is a little wider.
        tail: Length of the right-hand tail. 0 = Gaussian, above 0 = EMG.
        r2: Fit quality (0–1), set by fit_peak. Not needed to draw. Optional.

    Example:
        peak: PeakParameters = {"height": 9800, "center": 2.38, "fwhm": 0.021, "tail": 0.006}

    Documentation: https://phenological.github.io/quantion/?language=python#PeakParameters
    """
    r2: float


class PredictedRt(TypedDict):
    """PredictedRt · Results

    One predicted retention time from predict_rts.

    Fields:
        rt: Predicted retention time (minutes).
        uncertainty: How unsure the prediction is (minutes).

    Documentation: https://phenological.github.io/quantion/?language=python#PredictedRt
    """
    rt: float
    uncertainty: float


class ScanParamKind(str, Enum):
    """ScanParamKind · Enums

    What a row from get_scan_metadata holds.

    Fields:
        ATTRIBUTE: An attribute of the tag, such as id or defaultArrayLength.
        CV: A cvParam: a param from the controlled vocabulary, with an accession.
        USER: A userParam: a param the software that wrote the file named itself.

    Documentation: https://phenological.github.io/quantion/?language=python#ScanParamKind
    """
    ATTRIBUTE = "ATTRIBUTE"
    CV = "CV"
    USER = "USER"


class ScanParam(TypedDict):
    """ScanParam · Results

    One row from get_scan_metadata: an attribute, cvParam or userParam of one tag in a spectrum.

    Fields:
        path: Where the tag sits, from spectrum down. Repeated tags carry their position from 0, such as spectrum/precursorList/precursor[0]/selectedIonList/selectedIon[0].
        kind: What the row holds: ATTRIBUTE, CV or USER. See ScanParamKind.
        accession: Accession of a cvParam, such as MS:1000511. Missing for attributes and userParams.
        name: Name of the attribute or param, such as ms level.
        value: Value as text, such as 2. Missing when the file gives none.
        unit: Unit name, such as minute. Missing when there is none.
        unit_accession: Unit accession, such as UO:0000031. Missing when there is none.

    Documentation: https://phenological.github.io/quantion/?language=python#ScanParam
    """
    path: str
    kind: ScanParamKind
    accession: Optional[str]
    name: str
    value: Optional[str]
    unit: Optional[str]
    unit_accession: Optional[str]


class CompoundMatch(TypedDict):
    """CompoundMatch · Results

    One library spectrum that matches a feature in one file.

    Fields:
        label: Name of the compound, from the library param the model names as label.
        id: Id of the spectrum in its database.
        database: Database the id comes from.
        score: Average of the model vector similarity and the cosine (0–1).
        cosine: Cosine similarity of the cleaned spectra (0–1).
        mz: m/z values of the library spectrum.
        intensity: Intensities of the library spectrum.
        metadata: The other params of the library spectrum, by name.

    Documentation: https://phenological.github.io/quantion/?language=python#CompoundMatch
    """
    label: str
    id: str
    database: str
    score: float
    cosine: float
    mz: np.ndarray
    intensity: np.ndarray
    metadata: Dict[str, str]


class FileAnnotation(TypedDict):
    """FileAnnotation · Results

    What one file says about a feature.

    Fields:
        file: Position of the file in files, from 0.
        scan_index: The MS2 scan used, or none when the file has no peak or no MS2 scan close to it.
        matches: Best matches in this file, best first, at most matches.

    Documentation: https://phenological.github.io/quantion/?language=python#FileAnnotation
    """
    file: int
    scan_index: Optional[int]
    matches: List[CompoundMatch]


class FeatureAnnotation(TypedDict):
    """FeatureAnnotation · Results

    The name of one feature. The label is the compound that is the best match in the most files; ties go to the higher score.

    Fields:
        mz: m/z of the feature.
        rt: Retention time of the feature (minutes).
        label: Name of the compound, or none when no file has a match.
        id: Id in the database, from the agreeing file with the highest score.
        database: Database the id comes from.
        frequency: Share of the files whose best match is this compound (0–1).
        score: Median score over the agreeing files.
        cosine: Median cosine over the agreeing files.
        files: One FileAnnotation per file, in the order of files.

    Documentation: https://phenological.github.io/quantion/?language=python#FeatureAnnotation
    """
    mz: float
    rt: float
    label: Optional[str]
    id: Optional[str]
    database: Optional[str]
    frequency: float
    score: Optional[float]
    cosine: Optional[float]
    files: List[FileAnnotation]


class Scan(TypedDict):
    """Scan · Results

    One scan from get_scans: its retention time, centroided m/z and intensity values, and its details.

    Fields:
        index: Position of the scan in the spectrum list, from 0. Use it with get_scan_metadata.
        rt: Retention time (minutes).
        mz: m/z values.
        intensity: Intensities, same length as mz.
        level: MS level.
        precursor_mz: Precursor m/z (MS2 and higher).
        base_peak_mz: Base peak m/z.
        base_peak_intensity: Base peak intensity.
        total_ion_current: Total ion current.
        polarity: Polarity: 1 positive, 2 negative, 0 unknown.
        x: Imaging x position. 0 in non-imaging files.
        y: Imaging y position. 0 in non-imaging files.
        z: Imaging z position. 0 in non-imaging files.

    Documentation: https://phenological.github.io/quantion/?language=python#Scan
    """
    index: int
    rt: float
    mz: np.ndarray
    intensity: np.ndarray
    level: int
    precursor_mz: float
    base_peak_mz: float
    base_peak_intensity: float
    total_ion_current: float
    polarity: int
    x: int
    y: int
    z: int


class IonImage(TypedDict):
    """IonImage · Results

    An imaging map of one m/z from get_ion_image.

    Fields:
        width: Pixels across.
        height: Pixels down.
        min_x: x position of the first column.
        min_y: y position of the first row.
        min_z: Lowest z position.
        max_z: Highest z position.
        data: Summed intensity per pixel, row by row (width × height).
        counts: Scans per pixel.

    Documentation: https://phenological.github.io/quantion/?language=python#IonImage
    """
    width: int
    height: int
    min_x: int
    min_y: int
    min_z: int
    max_z: int
    data: np.ndarray
    counts: np.ndarray
