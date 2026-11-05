export type BinaryInput = Uint8Array | ArrayBuffer;
export type IonInput = BinaryInput | URL | string;

/**
 * TimeUnit · Enums
 *
 * Unit of the retention times you give to a function. Results are always in minutes.
 *
 * Fields:
 * - `MINUTES`: Minutes (default).
 * - `SECONDS`: Seconds.
 * @see {@link https://phenological.github.io/quantion/?language=js#TimeUnit | Documentation}
 */
export const TimeUnit = { MINUTES: "MINUTES", SECONDS: "SECONDS" } as const;
export type TimeUnit = (typeof TimeUnit)[keyof typeof TimeUnit];

/**
 * PeakShape · Enums
 *
 * Curve used to fit a peak.
 *
 * Fields:
 * - `GAUSSIAN`: Symmetric Gaussian.
 * - `EMG`: Exponentially modified Gaussian: a Gaussian with a tail on the right (default).
 * @see {@link https://phenological.github.io/quantion/?language=js#PeakShape | Documentation}
 */
export const PeakShape = { GAUSSIAN: "GAUSSIAN", EMG: "EMG" } as const;
export type PeakShape = (typeof PeakShape)[keyof typeof PeakShape];

/**
 * FromTo · Inputs
 *
 * A range: retention-time windows and m/z ranges.
 *
 * Fields:
 * - `from`: Start of the range.
 * - `to`: End of the range.
 *
 * @example
 * ```ts
 * const window: FromTo = { from: 1.0, to: 2.5 };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#FromTo | Documentation}
 */
export interface FromTo {
  from: number;
  to: number;
}

/**
 * Target · Inputs
 *
 * One compound. range limits the search in find_targets; missing or 0 searches the whole EIC. predict_rts and calibrate do not use range.
 *
 * Fields:
 * - `id`: Your name for it. Not used in the search. Optional.
 * - `mz`: m/z (Da).
 * - `rt`: Expected retention time.
 * - `range`: Half-width of the search window around rt. Missing or 0 = search the whole EIC. Optional.
 *
 * @example
 * ```ts
 * const target: Target = { id: "compound_1", mz: 260.103, rt: 2.383, range: 0.05 };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#Target | Documentation}
 */
export interface Target {
  id?: string;
  mz: number;
  rt: number;
  range?: number;
}

/**
 * Transition · Inputs
 *
 * One SRM transition.
 *
 * Fields:
 * - `id`: Your name for it. Optional.
 * - `precursor`: Precursor (Q1) m/z.
 * - `product`: Product (Q3) m/z.
 * - `rt`: Expected retention time.
 * - `range`: Half-width of the search window around rt.
 *
 * @example
 * ```ts
 * const transition: Transition = { id: "compound_1", precursor: 89.04768, product: 89.04768, rt: 2.385, range: 0.1 };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#Transition | Documentation}
 */
export interface Transition {
  id?: string;
  precursor: number;
  product: number;
  rt: number;
  range: number;
}

/**
 * Header · Inputs
 *
 * One HTTP header sent with an upload in write_ion. Values are never printed. A line break, or Host, Content-Length or Transfer-Encoding, gives an invalid_argument error.
 *
 * Fields:
 * - `name`: Name of the header, such as Authorization.
 * - `value`: Value of the header.
 *
 * @example
 * ```ts
 * const header: Header = { name: "Authorization", value: "Bearer <token>" };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#Header | Documentation}
 */
export interface Header {
  name: string;
  value: string;
}

export interface RtQuery {
  rt: number | FromTo;
}

export interface MzQuery {
  mz: number | FromTo;
}

/**
 * ScanQuery · Inputs
 *
 * Which scans get_scans returns.
 *
 * Fields:
 * - `{ rt: { from, to } }`: Every scan with rt in the range.
 * - `{ rt: 1.8 }`: The scan closest to this rt.
 * - `{ mz: { from, to } }`: Every scan with precursor m/z in the range.
 * - `{ mz: 445.12 }`: The scan whose precursor m/z is closest.
 *
 * @example
 * ```ts
 * const query: ScanQuery = { rt: { from: 1.0, to: 2.5 } };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#ScanQuery | Documentation}
 */
export type ScanQuery = RtQuery | MzQuery;

/**
 * FindPeaksOptions · Options
 *
 * Peak-picking settings. Every field is optional.
 *
 * Fields:
 * - `minIntegral`: Drop peaks with a smaller area. Off when not given. Default `off`.
 * - `minIntensity`: Drop peaks with a lower apex. Default `500`.
 * - `minPeakWidthPoints`: Drop peaks with fewer points. Default `3`.
 * - `noise`: Fixed noise level, used when auto_noise is false. Default `off`.
 * - `autoNoise`: Estimate the noise from the trace. Default `true`.
 * - `autoBaseline`: Subtract an airPLS baseline first. Default `true`.
 * - `lambda`: airPLS smoothness. Default `20`.
 * - `maxIterations`: Maximum airPLS iterations. Default `4`.
 * - `allowOverlap`: Let neighbouring peaks share points. Default `false`.
 * - `minSnr`: Minimum signal-to-noise ratio. Default `2`.
 * - `minR2`: Minimum fit quality of the peak shape (0–1). 0 keeps every peak. Default `0`.
 * - `shape`: Curve used for the fit. Default `"EMG"`.
 * - `kernelSize`: Smoothing window in points. 0 = no smoothing. Default `0`.
 * @see {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | Documentation}
 */
export interface FindPeaksOptions {
  minIntegral?: number;
  minIntensity?: number;
  minPeakWidthPoints?: number;
  noise?: number;
  autoNoise?: boolean;
  autoBaseline?: boolean;
  lambda?: number;
  maxIterations?: number;
  allowOverlap?: boolean;
  minSnr?: number;
  minR2?: number;
  shape?: PeakShape;
  kernelSize?: number;
}

/**
 * GetPeakOptions · Options
 *
 * Settings for get_peak.
 *
 * Fields:
 * - `range`: Half-width of the search window around rt. 0 searches the whole trace. Default `0`.
 * - `findPeaks`: Settings for peak picking. See {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | FindPeaksOptions}.
 * @see {@link https://phenological.github.io/quantion/?language=js#GetPeakOptions | Documentation}
 */
export interface GetPeakOptions {
  range?: number;
  findPeaks?: FindPeaksOptions;
}

/**
 * FitPeakOptions · Options
 *
 * Settings for fit_peak.
 *
 * Fields:
 * - `shape`: Curve to fit. Default `"EMG"`.
 * - `intensity`: Apex intensity to start the fit from. 0 = take it from the peak at rt. Default `0`.
 * @see {@link https://phenological.github.io/quantion/?language=js#FitPeakOptions | Documentation}
 */
export interface FitPeakOptions {
  shape?: PeakShape;
  intensity?: number;
}

/**
 * DrawPeaksOptions · Options
 *
 * Settings for draw_peaks.
 *
 * Fields:
 * - `from`: Start of the chromatogram. With to, 0 and 0 = fit the range around all the peaks. Default `0`.
 * - `to`: End of the chromatogram. Default `0`.
 * - `points`: Number of points drawn between from and to. Default `1000`.
 * @see {@link https://phenological.github.io/quantion/?language=js#DrawPeaksOptions | Documentation}
 */
export interface DrawPeaksOptions {
  from?: number;
  to?: number;
  points?: number;
}

/**
 * CalculateBaselineOptions · Options
 *
 * airPLS baseline settings for calculate_baseline.
 *
 * Fields:
 * - `lambda`: Smoothness. Larger = smoother. Default `20`.
 * - `maxIterations`: Maximum iterations. Default `4`.
 * @see {@link https://phenological.github.io/quantion/?language=js#CalculateBaselineOptions | Documentation}
 */
export interface CalculateBaselineOptions {
  lambda?: number;
  maxIterations?: number;
}

/**
 * CalculateEicOptions · Options
 *
 * Which scans are read and how wide the m/z window is. Used by calculate_eic, and as the eic group of find_targets, find_features and match_features. The wider of the ppm and absolute m/z windows is used.
 *
 * Fields:
 * - `from`: Start of the retention-time range read. Default `0`.
 * - `to`: End of the retention-time range read. The default reads to the end of the run. Default `Infinity`.
 * - `ppmTolerance`: Tolerance in ppm. Default `20`.
 * - `mzTolerance`: Absolute tolerance in Da. Default `0.005`.
 * - `timeUnit`: Unit of from, to and rt values given. Default `"MINUTES"`.
 * @see {@link https://phenological.github.io/quantion/?language=js#CalculateEicOptions | Documentation}
 */
export interface CalculateEicOptions {
  from?: number;
  to?: number;
  ppmTolerance?: number;
  mzTolerance?: number;
  timeUnit?: TimeUnit;
}

/**
 * GridOptions · Options
 *
 * The m/z grid that find_features and match_features search.
 *
 * Fields:
 * - `start`: Lowest m/z. Default `40`.
 * - `end`: Highest m/z. Default `1000`.
 * - `stepSize`: Step between grid m/z values (Da). Default `0.005`.
 * @see {@link https://phenological.github.io/quantion/?language=js#GridOptions | Documentation}
 */
export interface GridOptions {
  start?: number;
  end?: number;
  stepSize?: number;
}

/**
 * GroupingOptions · Options
 *
 * How match_features matches features across samples.
 *
 * Fields:
 * - `ppmTolerance`: Same feature when m/z is within this ppm…. Default `5`.
 * - `mzTolerance`: …or within this absolute m/z (Da). Default `0.0025`.
 * - `rtTolerance`: …and rt is within this many minutes. Default `0.05`.
 * - `minFrequency`: Keep a feature only if found in at least this fraction of samples (0–1). Default `0`.
 * @see {@link https://phenological.github.io/quantion/?language=js#GroupingOptions | Documentation}
 */
export interface GroupingOptions {
  ppmTolerance?: number;
  mzTolerance?: number;
  rtTolerance?: number;
  minFrequency?: number;
}

/**
 * FindTargetsOptions · Options
 *
 * Settings for find_targets.
 *
 * Fields:
 * - `eic`: EIC retention-time range (from, to), tolerances and time unit. See {@link https://phenological.github.io/quantion/?language=js#CalculateEicOptions | CalculateEicOptions}.
 * - `findPeaks`: Settings for peak picking. See {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | FindPeaksOptions}.
 * - `threads`: Number of threads. Default `1`.
 * @see {@link https://phenological.github.io/quantion/?language=js#FindTargetsOptions | Documentation}
 */
export interface FindTargetsOptions {
  eic?: CalculateEicOptions;
  findPeaks?: FindPeaksOptions;
  threads?: number;
}

/**
 * FindTransitionsOptions · Options
 *
 * Settings for find_transitions.
 *
 * Fields:
 * - `findPeaks`: Settings for peak picking. See {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | FindPeaksOptions}.
 * - `threads`: Number of threads. Default `1`.
 * @see {@link https://phenological.github.io/quantion/?language=js#FindTransitionsOptions | Documentation}
 */
export interface FindTransitionsOptions {
  findPeaks?: FindPeaksOptions;
  threads?: number;
}

/**
 * FindFeaturesOptions · Options
 *
 * Settings for find_features.
 *
 * Fields:
 * - `eic`: EIC retention-time range (from, to), tolerances and time unit. See {@link https://phenological.github.io/quantion/?language=js#CalculateEicOptions | CalculateEicOptions}.
 * - `grid`: The m/z grid. See {@link https://phenological.github.io/quantion/?language=js#GridOptions | GridOptions}.
 * - `findPeaks`: Settings for peak picking. See {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | FindPeaksOptions}.
 * - `threads`: Number of threads. Default `1`.
 * @see {@link https://phenological.github.io/quantion/?language=js#FindFeaturesOptions | Documentation}
 */
export interface FindFeaturesOptions {
  eic?: CalculateEicOptions;
  grid?: GridOptions;
  findPeaks?: FindPeaksOptions;
  threads?: number;
}

/**
 * DeisotopeFeaturesOptions · Options
 *
 * How deisotope_features links isotope peaks.
 *
 * Fields:
 * - `rtTolerance`: Same compound when rt is within this many minutes of M. Default `0.02`.
 * - `mzTolerance`: Isotope peak when m/z is within this many Da of the expected position. Default `0.005`.
 * @see {@link https://phenological.github.io/quantion/?language=js#DeisotopeFeaturesOptions | Documentation}
 */
export interface DeisotopeFeaturesOptions {
  rtTolerance?: number;
  mzTolerance?: number;
}

/**
 * MatchFeaturesOptions · Options
 *
 * Settings for match_features: the find_features settings plus grouping.
 *
 * Fields:
 * - `eic`: EIC retention-time range (from, to), tolerances and time unit. See {@link https://phenological.github.io/quantion/?language=js#CalculateEicOptions | CalculateEicOptions}.
 * - `grid`: The m/z grid. See {@link https://phenological.github.io/quantion/?language=js#GridOptions | GridOptions}.
 * - `grouping`: How features are matched across samples. See {@link https://phenological.github.io/quantion/?language=js#GroupingOptions | GroupingOptions}.
 * - `findPeaks`: Settings for peak picking. See {@link https://phenological.github.io/quantion/?language=js#FindPeaksOptions | FindPeaksOptions}.
 * - `threads`: Number of threads. Default `1`.
 * @see {@link https://phenological.github.io/quantion/?language=js#MatchFeaturesOptions | Documentation}
 */
export interface MatchFeaturesOptions {
  eic?: CalculateEicOptions;
  grid?: GridOptions;
  grouping?: GroupingOptions;
  findPeaks?: FindPeaksOptions;
  threads?: number;
}

/**
 * GetScanMetadataOptions · Options
 *
 * Settings for get_scan_metadata.
 *
 * Fields:
 * - `tags`: Which tags to keep, by name. A row is kept when the last tag of its path matches, so scan keeps spectrum/scanList/scan[0] but not its scanWindow. Empty keeps every tag. The names are: spectrum, referenceableParamGroupRef, spectrumDescription, scanList, scan, scanWindowList, scanWindow, precursorList, precursor, isolationWindow, selectedIonList, selectedIon, activation, productList, product, binaryDataArrayList, binaryDataArray. Default `[]`.
 * @see {@link https://phenological.github.io/quantion/?language=js#GetScanMetadataOptions | Documentation}
 */
export interface GetScanMetadataOptions {
  tags?: string[];
}

/**
 * GetScansOptions · Options
 *
 * Settings for get_scans.
 *
 * Fields:
 * - `level`: MS level. 0 = every level. Default `1`.
 * - `timeUnit`: Unit of rt in the query. Default `"MINUTES"`.
 * - `arrays`: Which arrays to read from each scan, as keys with accessions. Every other field of Scan always comes back. Default `{ key: "mz", accession: "MS:1000514" }, { key: "intensity", accession: "MS:1000515" }`. See {@link https://phenological.github.io/quantion/?language=js#ScanArray | ScanArray}.
 * @see {@link https://phenological.github.io/quantion/?language=js#GetScansOptions | Documentation}
 */
export interface GetScansOptions {
  level?: number;
  timeUnit?: TimeUnit;
  arrays?: ScanArray[];
}

/**
 * ScanArray · Options
 *
 * One array to read from each scan in get_scans.
 *
 * Fields:
 * - `key`: Name of the array in the result, as a field of the scan.
 * - `accession`: Accession of the array in the file, such as MS:1000514 for m/z.
 *
 * @example
 * ```ts
 * const array: ScanArray = { key: "mz", accession: "MS:1000514" };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#ScanArray | Documentation}
 */
export interface ScanArray {
  key: string;
  accession: string;
}

/**
 * ParseIonOptions · Options
 *
 * Settings for parse_ion.
 *
 * Fields:
 * - `maxCacheSize`: Cache size in bytes. 0 = no limit. Default `0`.
 * @see {@link https://phenological.github.io/quantion/?language=js#ParseIonOptions | Documentation}
 */
export interface ParseIonOptions {
  maxCacheSize?: number;
}

/**
 * WriteIonOptions · Options
 *
 * Settings for write_ion.
 *
 * Fields:
 * - `out`: Path or URL to write to. Not given = return the bytes. Default `undefined`.
 * - `overwrite`: Replace an existing file. Default `false`.
 * - `level`: Compression level, 0 to 22. Default `12`.
 * - `f32Compress`: Store intensities as 32-bit floats. Default `false`.
 * - `blockSize`: Size of each data block before compression, in MB. A whole number, 1 or more. Default `8`.
 * - `mzWindow`: Width of the m/z windows the spectra are split into, in Da, so a query reads only the windows it needs. A whole number, 0 or more; 0 keeps each spectrum whole. Default `20`.
 * - `headers`: Extra HTTP headers. Only with a URL. Default `[]`.
 * @see {@link https://phenological.github.io/quantion/?language=js#WriteIonOptions | Documentation}
 */
export interface WriteIonOptions {
  out?: string | URL;
  overwrite?: boolean;
  level?: number;
  f32Compress?: boolean;
  blockSize?: number;
  mzWindow?: number;
  headers?: Header[];
}

/**
 * WriteMzmlOptions · Options
 *
 * Settings for write_mzml.
 *
 * Fields:
 * - `out`: Path to write to. Not given = return the bytes. Default `undefined`.
 * - `overwrite`: Replace an existing file. Default `false`.
 * @see {@link https://phenological.github.io/quantion/?language=js#WriteMzmlOptions | Documentation}
 */
export interface WriteMzmlOptions {
  out?: string;
  overwrite?: boolean;
}

/**
 * GetIonImageOptions · Options
 *
 * Settings for get_ion_image.
 *
 * Fields:
 * - `tolerance`: Absolute m/z tolerance (Da). Default `0.125`.
 * - `level`: MS level. Default `1`.
 * - `onProgress`: Progress callback: fetched bytes, total bytes, bytes held. Default `undefined`.
 * @see {@link https://phenological.github.io/quantion/?language=js#GetIonImageOptions | Documentation}
 */
export interface GetIonImageOptions {
  tolerance?: number;
  level?: number;
  onProgress?: (fetched: number, total: number, heldBytes: number) => void;
}

/**
 * Peak · Results
 *
 * One chromatographic peak. No peak gives every field 0.
 *
 * Fields:
 * - `rt`: Apex retention time (minutes).
 * - `from`: Start of the peak.
 * - `to`: End of the peak.
 * - `intensity`: Apex intensity.
 * - `integral`: Area of the peak.
 * - `nPoints`: Points across the peak.
 * @see {@link https://phenological.github.io/quantion/?language=js#Peak | Documentation}
 */
export interface Peak {
  rt: number;
  from: number;
  to: number;
  intensity: number;
  integral: number;
  nPoints: number;
}

/**
 * Feature · Results
 *
 * A peak at an m/z. match_features adds frequency.
 *
 * Fields:
 * - `mz`: m/z (Da).
 * - `rt`: Apex retention time (minutes).
 * - `from`: Start of the peak.
 * - `to`: End of the peak.
 * - `intensity`: Apex intensity.
 * - `integral`: Area of the peak.
 * - `nPoints`: Points across the peak.
 * - `frequency`: match_features only: fraction of samples where it was found (0–1). Optional.
 * @see {@link https://phenological.github.io/quantion/?language=js#Feature | Documentation}
 */
export interface Feature {
  mz: number;
  rt: number;
  from: number;
  to: number;
  intensity: number;
  integral: number;
  nPoints: number;
  frequency?: number;
}

/**
 * DataXY · Results
 *
 * A trace: x = retention times, y = intensities. Returned by calculate_eic, taken by find_peaks.
 *
 * Fields:
 * - `x`: Retention times (minutes).
 * - `y`: Intensities.
 * @see {@link https://phenological.github.io/quantion/?language=js#DataXY | Documentation}
 */
export interface DataXY {
  x: Float64Array;
  y: Float64Array;
}

/**
 * NoiseLevel · Results
 *
 * Result of find_noise_level.
 *
 * Fields:
 * - `width`: Window width (points) the estimate came from.
 * - `intensity`: Noise intensity.
 * @see {@link https://phenological.github.io/quantion/?language=js#NoiseLevel | Documentation}
 */
export interface NoiseLevel {
  width: number;
  intensity: number;
}

/**
 * PeakParameters · Results
 *
 * One peak curve: returned by fit_peak, or written by hand to simulate. draw_peaks takes a list of them. tail = 0 is a Gaussian, tail above 0 an EMG.
 *
 * Fields:
 * - `height`: Height of the curve at its apex.
 * - `center`: Retention time of the apex (minutes).
 * - `fwhm`: Full width at half maximum of the Gaussian part (minutes). With a tail, the drawn peak is a little wider.
 * - `tail`: Length of the right-hand tail. 0 = Gaussian, above 0 = EMG.
 * - `r2`: Fit quality (0–1), set by fit_peak. Not needed to draw. Optional.
 *
 * @example
 * ```ts
 * const peak: PeakParameters = { height: 9800, center: 2.38, fwhm: 0.021, tail: 0.006 };
 * ```
 * @see {@link https://phenological.github.io/quantion/?language=js#PeakParameters | Documentation}
 */
export interface PeakParameters {
  height: number;
  center: number;
  fwhm: number;
  tail: number;
  r2?: number;
}

/**
 * PredictedRt · Results
 *
 * One predicted retention time from predict_rts.
 *
 * Fields:
 * - `rt`: Predicted retention time (minutes).
 * - `uncertainty`: How unsure the prediction is (minutes).
 * @see {@link https://phenological.github.io/quantion/?language=js#PredictedRt | Documentation}
 */
export interface PredictedRt {
  rt: number;
  uncertainty: number;
}

/**
 * ScanParamKind · Enums
 *
 * What a row from get_scan_metadata holds.
 *
 * Fields:
 * - `ATTRIBUTE`: An attribute of the tag, such as id or defaultArrayLength.
 * - `CV`: A cvParam: a param from the controlled vocabulary, with an accession.
 * - `USER`: A userParam: a param the software that wrote the file named itself.
 * @see {@link https://phenological.github.io/quantion/?language=js#ScanParamKind | Documentation}
 */
export const ScanParamKind = {
  ATTRIBUTE: "ATTRIBUTE",
  CV: "CV",
  USER: "USER",
} as const;
export type ScanParamKind = (typeof ScanParamKind)[keyof typeof ScanParamKind];

/**
 * ScanParam · Results
 *
 * One row from get_scan_metadata: an attribute, cvParam or userParam of one tag in a spectrum.
 *
 * Fields:
 * - `path`: Where the tag sits, from spectrum down. Repeated tags carry their position from 0, such as spectrum/precursorList/precursor[0]/selectedIonList/selectedIon[0].
 * - `kind`: What the row holds: ATTRIBUTE, CV or USER. See ScanParamKind.
 * - `accession`: Accession of a cvParam, such as MS:1000511. Missing for attributes and userParams.
 * - `name`: Name of the attribute or param, such as ms level.
 * - `value`: Value as text, such as 2. Missing when the file gives none.
 * - `unit`: Unit name, such as minute. Missing when there is none.
 * - `unitAccession`: Unit accession, such as UO:0000031. Missing when there is none.
 * @see {@link https://phenological.github.io/quantion/?language=js#ScanParam | Documentation}
 */
export interface ScanParam {
  path: string;
  kind: ScanParamKind;
  accession: string | null;
  name: string;
  value: string | null;
  unit: string | null;
  unitAccession: string | null;
}

/**
 * Scan · Results
 *
 * One scan from get_scans: its retention time, centroided m/z and intensity values, and its details.
 *
 * Fields:
 * - `index`: Position of the scan in the spectrum list, from 0. Use it with get_scan_metadata.
 * - `rt`: Retention time (minutes).
 * - `mz`: m/z values.
 * - `intensity`: Intensities, same length as mz.
 * - `level`: MS level.
 * - `precursorMz`: Precursor m/z (MS2 and higher).
 * - `basePeakMz`: Base peak m/z.
 * - `basePeakIntensity`: Base peak intensity.
 * - `totalIonCurrent`: Total ion current.
 * - `polarity`: Polarity: 1 positive, 2 negative, 0 unknown.
 * - `x`: Imaging x position. 0 in non-imaging files.
 * - `y`: Imaging y position. 0 in non-imaging files.
 * - `z`: Imaging z position. 0 in non-imaging files.
 * @see {@link https://phenological.github.io/quantion/?language=js#Scan | Documentation}
 */
export interface Scan {
  index: number;
  rt: number;
  mz: Float64Array;
  intensity: Float64Array;
  level: number;
  precursorMz: number;
  basePeakMz: number;
  basePeakIntensity: number;
  totalIonCurrent: number;
  polarity: number;
  x: number;
  y: number;
  z: number;
  [array: string]: number | Float64Array;
}

/**
 * IonImage · Results
 *
 * An imaging map of one m/z from get_ion_image.
 *
 * Fields:
 * - `width`: Pixels across.
 * - `height`: Pixels down.
 * - `minX`: x position of the first column.
 * - `minY`: y position of the first row.
 * - `minZ`: Lowest z position.
 * - `maxZ`: Highest z position.
 * - `data`: Summed intensity per pixel, row by row (width × height).
 * - `counts`: Scans per pixel.
 * @see {@link https://phenological.github.io/quantion/?language=js#IonImage | Documentation}
 */
export interface IonImage {
  width: number;
  height: number;
  minX: number;
  minY: number;
  minZ: number;
  maxZ: number;
  data: Float64Array;
  counts: Uint32Array;
}
