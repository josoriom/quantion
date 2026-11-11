use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
};

use ionic::{
    ByteRangesOptions, IonReader,
    source::{ByteRange, merge_ranges},
};

use crate::{
    annotations::AnnotateFeaturesOptions,
    files::{CalculateEicOptions, ScanParamKind},
    sample_file::SampleFile,
    targets::FindTargetsOptions,
    utilities::{
        calculate_eic::{FastError, TimeUnit, lower_bound, upper_bound},
        clean_spectrum::{Spectrum, clean_spectrum},
        find_features::Feature,
        find_targets::{find_targets, plan_targets_ranges},
        get_cosine::get_cosine,
        get_scans::{
            CentroidScan, find_wanted_scans, get_scans, plan_scan_ranges, whole_scan_mz_range,
        },
        ion::ScanSource,
        spectrum_model::{SpectrumModel, SpectrumModelError},
        structs::{FromTo, Position, ScanQuery, Target},
    },
};

const MS2_LEVEL: u8 = 2;
const POSITIVE: u8 = 1;
const EIC_HALF_WINDOW: f64 = 0.5;
const ABOVE_PRECURSOR_LIMIT: f64 = 1.0;
const MZ_ARRAY: u32 = 1_000_514;
const INTENSITY_ARRAY: u32 = 1_000_515;

/// **FeatureAnnotation** · Results
///
/// The name of one feature. The label is the compound that is the best match in the most files; ties go to the higher score.
///
/// Fields:
/// - `mz`: m/z of the feature.
/// - `rt`: Retention time of the feature (minutes).
/// - `label`: Name of the compound, or none when no file has a match. Optional.
/// - `id`: Id in the database, from the agreeing file with the highest score. Optional.
/// - `database`: Database the id comes from. Optional.
/// - `frequency`: Share of the files whose best match is this compound (0–1).
/// - `score`: Median score over the agreeing files. Optional.
/// - `cosine`: Median cosine over the agreeing files. Optional.
/// - `files`: One FileAnnotation per file, in the order of files.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FeatureAnnotation)
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureAnnotation {
    pub mz: f64,
    pub rt: f64,
    pub label: Option<String>,
    pub id: Option<String>,
    pub database: Option<String>,
    pub frequency: f64,
    pub score: Option<f64>,
    pub cosine: Option<f64>,
    pub files: Vec<FileAnnotation>,
}

/// **FileAnnotation** · Results
///
/// What one file says about a feature.
///
/// Fields:
/// - `file`: Position of the file in files, from 0.
/// - `scan_index`: The MS2 scan used, or none when the file has no peak or no MS2 scan close to it. Optional.
/// - `matches`: Best matches in this file, best first, at most matches.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#FileAnnotation)
#[derive(Clone, Debug, PartialEq)]
pub struct FileAnnotation {
    pub file: usize,
    pub scan_index: Option<usize>,
    pub matches: Vec<CompoundMatch>,
}

/// **CompoundMatch** · Results
///
/// One library spectrum that matches a feature in one file.
///
/// Fields:
/// - `label`: Name of the compound, from the library param the model names as label.
/// - `id`: Id of the spectrum in its database.
/// - `database`: Database the id comes from.
/// - `score`: Average of the model vector similarity and the cosine (0–1).
/// - `cosine`: Cosine similarity of the cleaned spectra (0–1).
/// - `mz`: m/z values of the library spectrum.
/// - `intensity`: Intensities of the library spectrum.
/// - `metadata`: The other params of the library spectrum, by name.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#CompoundMatch)
#[derive(Clone, Debug, PartialEq)]
pub struct CompoundMatch {
    pub label: String,
    pub id: String,
    pub database: String,
    pub score: f64,
    pub cosine: f64,
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub(crate) enum AnnotationError {
    ReadFailed(FastError),
    Model(SpectrumModelError),
    MissingVector {
        spectrum: usize,
        accession: u32,
    },
    VectorSize {
        spectrum: usize,
        found: usize,
        expected: usize,
    },
    MissingCompound {
        spectrum: usize,
        param: String,
    },
}

impl fmt::Display for AnnotationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed(error) => write!(formatter, "{error}"),
            Self::Model(error) => write!(formatter, "{error}"),
            Self::MissingVector {
                spectrum,
                accession,
            } => write!(formatter, "spectrum {spectrum} has no array MS:{accession}"),
            Self::VectorSize {
                spectrum,
                found,
                expected,
            } => {
                write!(
                    formatter,
                    "spectrum {spectrum} has a vector of {found} values; the model gives {expected}"
                )
            }
            Self::MissingCompound { spectrum, param } => {
                write!(formatter, "spectrum {spectrum} has no userParam {param}")
            }
        }
    }
}

impl From<FastError> for AnnotationError {
    fn from(error: FastError) -> Self {
        Self::ReadFailed(error)
    }
}

impl From<ionic::IonError> for AnnotationError {
    fn from(error: ionic::IonError) -> Self {
        Self::ReadFailed(FastError::from(error))
    }
}

impl From<SpectrumModelError> for AnnotationError {
    fn from(error: SpectrumModelError) -> Self {
        Self::Model(error)
    }
}

pub(crate) fn annotate_features(
    files: &mut [&mut SampleFile],
    features: &[Feature],
    library: &mut IonReader,
    model: &SpectrumModel,
    options: &AnnotateFeaturesOptions,
) -> Result<Vec<FeatureAnnotation>, AnnotationError> {
    let file_count = files.len();
    let mut feature_scans: Vec<FeatureScan> = Vec::new();
    for (position, file) in files.iter_mut().enumerate() {
        feature_scans.extend(find_feature_scans(file, position, features, options)?);
    }
    feature_scans.sort_by_key(|feature_scan| (feature_scan.feature, feature_scan.file));
    let spectra: Vec<Spectrum> = feature_scans
        .iter()
        .map(|feature_scan| feature_scan.spectrum.clone())
        .collect();
    let vectors = model.get_vectors(&spectra)?;
    drop(spectra);
    let index = LibraryIndex::read(library);
    let mut results = Vec::with_capacity(features.len());
    let mut next_scan = 0;
    for (position, feature) in features.iter().enumerate() {
        let mut scorer = FeatureScorer {
            library,
            index: &index,
            model,
            options,
            candidates: HashMap::new(),
        };
        let mut file_matches: Vec<Vec<ScanMatch>> = vec![Vec::new(); file_count];
        let mut scan_indexes: Vec<Option<usize>> = vec![None; file_count];
        while next_scan < feature_scans.len() && feature_scans[next_scan].feature == position {
            let feature_scan = &feature_scans[next_scan];
            scan_indexes[feature_scan.file] = Some(feature_scan.scan_index);
            file_matches[feature_scan.file] =
                scorer.score_scan(feature_scan, &vectors[next_scan])?;
            next_scan += 1;
        }
        results.push(scorer.get_annotation(feature, &file_matches, &scan_indexes));
    }
    Ok(results)
}

fn find_feature_scans(
    file: &mut SampleFile,
    position: usize,
    features: &[Feature],
    options: &AnnotateFeaturesOptions,
) -> Result<Vec<FeatureScan>, AnnotationError> {
    let mut feature_scans = Vec::new();
    for (feature_position, feature) in features.iter().enumerate() {
        let Some(apex) = find_apex(file, feature, options)? else {
            continue;
        };
        if let Some(scan) = find_closest_scan(file, feature, apex, options) {
            feature_scans.push(FeatureScan {
                feature: feature_position,
                file: position,
                scan_index: scan.index,
                precursor: scan.metadata.selected_ion_mz,
                spectrum: clean_spectrum(
                    Spectrum {
                        mz: scan.mz.to_vec(),
                        intensity: scan.intensity.to_vec(),
                    },
                    options.min_relative_intensity,
                ),
            });
        }
    }
    Ok(feature_scans)
}

struct FeatureScan {
    feature: usize,
    file: usize,
    scan_index: usize,
    precursor: f64,
    spectrum: Spectrum,
}

fn find_apex(
    file: &mut SampleFile,
    feature: &Feature,
    options: &AnnotateFeaturesOptions,
) -> Result<Option<f64>, FastError> {
    let target = get_target(feature, options);
    let peaks = find_targets(
        &mut file.reader(),
        &[target],
        get_eic_window(feature),
        FindTargetsOptions::default(),
    )?;
    Ok(peaks
        .first()
        .filter(|peak| peak.intensity > 0.0)
        .map(|peak| peak.rt))
}

fn find_closest_scan(
    file: &mut SampleFile,
    feature: &Feature,
    apex: f64,
    options: &AnnotateFeaturesOptions,
) -> Option<CentroidScan> {
    let query = ScanQuery {
        rt: Position::Closest(apex),
        mz: Position::Range(options.get_precursor_window(feature.mz)),
    };
    let scans = get_scans(file, query, TimeUnit::MINUTES, MS2_LEVEL);
    scans.into_iter().next().filter(|scan| {
        scan.metadata.polarity == POSITIVE && (scan.rt - apex).abs() <= options.rt_range
    })
}

struct FeatureScorer<'a> {
    library: &'a mut IonReader,
    index: &'a LibraryIndex,
    model: &'a SpectrumModel,
    options: &'a AnnotateFeaturesOptions,
    candidates: HashMap<usize, Candidate>,
}

impl FeatureScorer<'_> {
    fn score_scan(
        &mut self,
        feature_scan: &FeatureScan,
        vector: &[f64],
    ) -> Result<Vec<ScanMatch>, AnnotationError> {
        let mut spectrum = feature_scan.spectrum.clone();
        spectrum.sort_by_mz();
        spectrum.keep_below(feature_scan.precursor + ABOVE_PRECURSOR_LIMIT);
        let mut matches: Vec<ScanMatch> = Vec::new();
        for library_spectrum in self
            .index
            .find(self.options.get_precursor_window(feature_scan.precursor))
        {
            if !self.candidates.contains_key(&library_spectrum.index) {
                let candidate = read_candidate(self.library, &library_spectrum, self.model)?;
                self.candidates.insert(library_spectrum.index, candidate);
            }
            let candidate = &self.candidates[&library_spectrum.index];
            if candidate.vector.len() != vector.len() {
                return Err(AnnotationError::VectorSize {
                    spectrum: library_spectrum.index,
                    found: candidate.vector.len(),
                    expected: vector.len(),
                });
            }
            let vector_score: f64 = candidate
                .vector
                .iter()
                .zip(vector)
                .map(|(left, right)| left * right)
                .sum();
            let cosine = get_cosine(&spectrum, &candidate.cosine_spectrum);
            let found = ScanMatch {
                compound: candidate.compound.clone(),
                score: (vector_score + cosine) / 2.0,
                cosine,
                library_index: library_spectrum.index,
            };
            keep_best(&mut matches, found);
        }
        matches.sort_by(|left, right| right.score.total_cmp(&left.score));
        matches.truncate(self.options.matches);
        Ok(matches)
    }

    fn get_annotation(
        &self,
        feature: &Feature,
        file_matches: &[Vec<ScanMatch>],
        scan_indexes: &[Option<usize>],
    ) -> FeatureAnnotation {
        let files = file_matches
            .iter()
            .zip(scan_indexes)
            .enumerate()
            .map(|(file, (matches, scan_index))| FileAnnotation {
                file,
                scan_index: *scan_index,
                matches: matches
                    .iter()
                    .map(|found| self.get_compound_match(found))
                    .collect(),
            })
            .collect();
        let tops: Vec<&ScanMatch> = file_matches
            .iter()
            .filter_map(|matches| matches.first())
            .collect();
        let Some(compound) = get_most_frequent_compound(&tops) else {
            return FeatureAnnotation {
                mz: feature.mz,
                rt: feature.rt,
                label: None,
                id: None,
                database: None,
                frequency: 0.0,
                score: None,
                cosine: None,
                files,
            };
        };
        let agreeing: Vec<&ScanMatch> = tops
            .iter()
            .copied()
            .filter(|found| found.compound == compound)
            .collect();
        let best = agreeing.iter().copied().fold(agreeing[0], |best, found| {
            if found.score > best.score {
                found
            } else {
                best
            }
        });
        let candidate = &self.candidates[&best.library_index];
        FeatureAnnotation {
            mz: feature.mz,
            rt: feature.rt,
            label: Some(candidate.label.clone()),
            id: Some(candidate.id.clone()),
            database: Some(candidate.database.clone()),
            frequency: agreeing.len() as f64 / file_matches.len() as f64,
            score: Some(get_median(
                agreeing.iter().map(|found| found.score).collect(),
            )),
            cosine: Some(get_median(
                agreeing.iter().map(|found| found.cosine).collect(),
            )),
            files,
        }
    }

    fn get_compound_match(&self, found: &ScanMatch) -> CompoundMatch {
        let candidate = &self.candidates[&found.library_index];
        CompoundMatch {
            label: candidate.label.clone(),
            id: candidate.id.clone(),
            database: candidate.database.clone(),
            score: found.score,
            cosine: found.cosine,
            mz: candidate.spectrum.mz.clone(),
            intensity: candidate.spectrum.intensity.clone(),
            metadata: candidate.metadata.clone(),
        }
    }
}

fn keep_best(matches: &mut Vec<ScanMatch>, found: ScanMatch) {
    if !found.score.is_finite() {
        return;
    }
    match matches
        .iter_mut()
        .find(|kept| kept.compound == found.compound)
    {
        Some(kept) if found.score > kept.score => *kept = found,
        Some(_) => {}
        None => matches.push(found),
    }
}

fn get_most_frequent_compound(tops: &[&ScanMatch]) -> Option<String> {
    let mut counts: Vec<(&str, usize, f64)> = Vec::new();
    for found in tops {
        match counts.iter_mut().find(|count| count.0 == found.compound) {
            Some(count) => {
                count.1 += 1;
                count.2 = count.2.max(found.score);
            }
            None => counts.push((&found.compound, 1, found.score)),
        }
    }
    counts
        .iter()
        .fold(
            None,
            |best: Option<&(&str, usize, f64)>, count| match best {
                Some(best) if (best.1, best.2) >= (count.1, count.2) => Some(best),
                _ => Some(count),
            },
        )
        .map(|count| count.0.to_string())
}

fn get_median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

#[derive(Clone)]
struct ScanMatch {
    compound: String,
    score: f64,
    cosine: f64,
    library_index: usize,
}

fn read_candidate(
    library: &mut IonReader,
    spectrum: &LibrarySpectrum,
    model: &SpectrumModel,
) -> Result<Candidate, AnnotationError> {
    let mut vector = Vec::new();
    if !library.load_array(spectrum.index, model.vector_accession, &mut vector) {
        return Err(AnnotationError::MissingVector {
            spectrum: spectrum.index,
            accession: model.vector_accession,
        });
    }
    let mut peaks = Spectrum::default();
    if !library.load_scan(spectrum.index, &mut peaks.mz, &mut peaks.intensity) {
        peaks = Spectrum::default();
    }
    peaks.sort_by_mz();
    let mut cosine_spectrum = peaks.clone();
    cosine_spectrum.keep_below(spectrum.precursor + ABOVE_PRECURSOR_LIMIT);
    let mut metadata = BTreeMap::new();
    for row in library.load_metadata(spectrum.index)? {
        if row.kind == ScanParamKind::USER {
            metadata
                .entry(row.name)
                .or_insert_with(|| row.value.unwrap_or_default());
        }
    }
    let compound = metadata
        .get(&model.compound_param)
        .cloned()
        .ok_or_else(|| AnnotationError::MissingCompound {
            spectrum: spectrum.index,
            param: model.compound_param.clone(),
        })?;
    let label = metadata.remove(&model.label_param).unwrap_or_default();
    let id = metadata.remove(&model.id_param).unwrap_or_default();
    let database = metadata.remove(&model.database_param).unwrap_or_default();
    Ok(Candidate {
        compound,
        label,
        id,
        database,
        vector,
        spectrum: peaks,
        cosine_spectrum,
        metadata,
    })
}

struct Candidate {
    compound: String,
    label: String,
    id: String,
    database: String,
    vector: Vec<f64>,
    spectrum: Spectrum,
    cosine_spectrum: Spectrum,
    metadata: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug)]
struct LibrarySpectrum {
    index: usize,
    precursor: f64,
}

struct LibraryIndex {
    precursors: Vec<f64>,
    indexes: Vec<usize>,
}

impl LibraryIndex {
    fn read(library: &mut IonReader) -> Self {
        let mut spectra = Vec::new();
        library.for_each_summary(&mut |index, summary| {
            if summary.ms_level == MS2_LEVEL && summary.selected_ion_mz.is_finite() {
                spectra.push(LibrarySpectrum {
                    index,
                    precursor: summary.selected_ion_mz,
                });
            }
        });
        spectra.sort_by(|left, right| {
            left.precursor
                .total_cmp(&right.precursor)
                .then(left.index.cmp(&right.index))
        });
        Self {
            precursors: spectra.iter().map(|spectrum| spectrum.precursor).collect(),
            indexes: spectra.iter().map(|spectrum| spectrum.index).collect(),
        }
    }

    fn find(&self, window: FromTo) -> Vec<LibrarySpectrum> {
        let first = lower_bound(&self.precursors, window.from);
        let last = upper_bound(&self.precursors, window.to).max(first);
        (first..last)
            .map(|position| LibrarySpectrum {
                index: self.indexes[position],
                precursor: self.precursors[position],
            })
            .collect()
    }
}

pub(crate) fn plan_sample_ranges(
    ion: &mut IonReader,
    features: &[Feature],
    options: &AnnotateFeaturesOptions,
) -> Result<Vec<ByteRange>, FastError> {
    ion.require_bounds()?;
    let mut ranges = Vec::new();
    for feature in features {
        let window = get_eic_window(feature);
        let target = get_target(feature, options);
        ranges.extend(plan_targets_ranges(
            ion,
            &[target],
            window.from,
            window.to,
            CalculateEicOptions::default(),
        )?);
        let query = get_ms2_query(feature, options);
        ranges.extend(plan_scan_ranges(
            ion,
            query,
            TimeUnit::MINUTES,
            MS2_LEVEL,
            &ByteRangesOptions::default(),
        )?);
    }
    merge_ranges(&mut ranges, 0);
    Ok(ranges)
}

pub(crate) fn plan_library_ranges(
    library: &mut IonReader,
    files: &mut [&mut SampleFile],
    features: &[Feature],
    options: &AnnotateFeaturesOptions,
    vector_accession: u32,
) -> Result<Vec<ByteRange>, FastError> {
    library.require_bounds()?;
    let index = LibraryIndex::read(library);
    let mut wanted = BTreeSet::new();
    for file in files.iter_mut() {
        for feature in features {
            let scans = find_wanted_scans(
                *file,
                get_ms2_query(feature, options),
                TimeUnit::MINUTES,
                MS2_LEVEL,
            )
            .scans;
            for (_, scan) in scans {
                let candidates = index.find(options.get_precursor_window(scan.selected_ion_mz));
                wanted.extend(candidates.iter().map(|spectrum| spectrum.index));
            }
        }
    }
    let arrays = ByteRangesOptions {
        arrays: vec![MZ_ARRAY, INTENSITY_ARRAY, vector_accession],
    };
    let mut ranges = Vec::new();
    for spectrum in wanted {
        ranges.extend(library.byte_ranges(spectrum, whole_scan_mz_range(), &arrays)?);
        ranges.extend(library.spectrum_metadata_byte_ranges(spectrum)?);
    }
    merge_ranges(&mut ranges, 0);
    Ok(ranges)
}

fn get_target(feature: &Feature, options: &AnnotateFeaturesOptions) -> Target {
    Target {
        id: None,
        mz: feature.mz,
        rt: feature.rt,
        range: Some(options.rt_range),
    }
}

fn get_eic_window(feature: &Feature) -> FromTo {
    FromTo {
        from: feature.rt - EIC_HALF_WINDOW,
        to: feature.rt + EIC_HALF_WINDOW,
    }
}

fn get_ms2_query(feature: &Feature, options: &AnnotateFeaturesOptions) -> ScanQuery {
    let reach = options.rt_range + options.rt_range.min(EIC_HALF_WINDOW);
    ScanQuery {
        rt: Position::Range(FromTo {
            from: feature.rt - reach,
            to: feature.rt + reach,
        }),
        mz: Position::Range(options.get_precursor_window(feature.mz)),
    }
}
