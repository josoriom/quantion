use core::ffi::c_int;

use super::{
    Buf, ParsedFile, clear_buf,
    model::ModelHandle,
    public::{FIELD_SEPARATOR, get_open_file, read_targets_at, read_values, remember, run},
    write_buf,
};
use crate::{
    AnnotateFeaturesOptions, Feature, QuantionError, SampleFile,
    annotations::{annotate_with_library, check_features, check_files, from_spectrum_model_error},
    files::from_fast_error,
    sample_file::FileSource,
    utilities::{
        annotate_features::{FeatureAnnotation, plan_library_ranges, plan_sample_ranges},
        bridge::{Column, build_record_bridge, *},
        spectrum_model::SpectrumModel,
    },
};

const FUNCTION: &str = "annotate_features";

fn read_options(
    matches: usize,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    rt_range: f64,
    min_relative_intensity: f64,
) -> Result<AnnotateFeaturesOptions, QuantionError> {
    let options = AnnotateFeaturesOptions {
        library: String::new(),
        model: String::new(),
        matches,
        ppm_tolerance,
        mz_tolerance,
        rt_range,
        min_relative_intensity,
    };
    options.check_settings(FUNCTION)?;
    Ok(options)
}

fn read_features(
    mzs: *const f64,
    rts: *const f64,
    count: usize,
) -> Result<Vec<Feature>, QuantionError> {
    if count > 0 && (mzs.is_null() || rts.is_null()) {
        return Err(QuantionError::internal(FUNCTION, "the features are null"));
    }
    let features: Vec<Feature> = read_targets_at(mzs, rts, count)
        .iter()
        .map(|target| Feature::new(target.mz, target.rt))
        .collect();
    check_features(FUNCTION, &features)?;
    Ok(features)
}

fn read_files<'a>(
    library: *mut ParsedFile,
    files: *const *mut ParsedFile,
    count: usize,
) -> Result<Vec<&'a mut SampleFile>, QuantionError> {
    check_files(FUNCTION, count)?;
    if files.is_null() {
        return Err(QuantionError::internal(FUNCTION, "the files are null"));
    }
    let handles = read_values(files, count);
    for (position, handle) in handles.iter().enumerate() {
        if *handle == library {
            return Err(QuantionError::invalid(
                FUNCTION,
                format!("files[{position}]"),
                "must not be the library",
            ));
        }
        if handles[..position].contains(handle) {
            return Err(QuantionError::invalid(
                FUNCTION,
                format!("files[{position}]"),
                "is given twice",
            ));
        }
    }
    handles
        .iter()
        .enumerate()
        .map(|(position, handle)| get_open_file(FUNCTION, *handle, &format!("files[{position}]")))
        .collect()
}

fn read_model(model: *const ModelHandle) -> Result<SpectrumModel, QuantionError> {
    if model.is_null() {
        return Err(QuantionError::invalid(
            FUNCTION,
            "model",
            "is not open; it was disposed or never opened",
        ));
    }
    SpectrumModel::read(unsafe { &(*model).model })
        .map_err(|error| from_spectrum_model_error(FUNCTION, error))
}

/// Match each feature against the library. `files` holds `file_count` open samples.
///
/// # Safety
/// `library` and every handle in `files` must be valid `ParsedFile`s; `model` a valid model.
/// `mzs` and `rts` must point to `feature_count` values. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn annotate_features(
    library: *mut ParsedFile,
    model: *const ModelHandle,
    files: *const *mut ParsedFile,
    file_count: usize,
    mzs: *const f64,
    rts: *const f64,
    feature_count: usize,
    matches: usize,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    rt_range: f64,
    min_relative_intensity: f64,
    out_features: *mut Buf,
    out_files: *mut Buf,
    out_matches: *mut Buf,
) -> c_int {
    if out_features.is_null() || out_files.is_null() || out_matches.is_null() {
        return remember(QuantionError::internal(
            FUNCTION,
            "an output buffer is null",
        ));
    }
    clear_buf(out_features);
    clear_buf(out_files);
    clear_buf(out_matches);
    run(FUNCTION, || {
        let options = read_options(
            matches,
            ppm_tolerance,
            mz_tolerance,
            rt_range,
            min_relative_intensity,
        )?;
        let mut samples = read_files(library, files, file_count)?;
        let features = read_features(mzs, rts, feature_count)?;
        let found = if features.is_empty() {
            Vec::new()
        } else {
            let model = read_model(model)?;
            let library = get_open_file(FUNCTION, library, "library")?;
            annotate_with_library(FUNCTION, library, &model, &mut samples, &features, &options)?
        };
        let could_not_pack =
            || QuantionError::internal(FUNCTION, "the annotations could not be packed");
        write_buf(
            out_features,
            build_features_bridge(&found).ok_or_else(could_not_pack)?,
        );
        write_buf(
            out_files,
            build_files_bridge(&found).ok_or_else(could_not_pack)?,
        );
        write_buf(
            out_matches,
            build_matches_bridge(&found).ok_or_else(could_not_pack)?,
        );
        Ok(())
    })
}

/// Plan the byte ranges `annotate_features` reads from one sample.
///
/// # Safety
/// `file` must be a valid `ParsedFile`. `mzs` and `rts` must point to `feature_count` values.
/// `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_annotate_samples(
    file: *mut ParsedFile,
    mzs: *const f64,
    rts: *const f64,
    feature_count: usize,
    matches: usize,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    rt_range: f64,
    min_relative_intensity: f64,
    out: *mut Buf,
) -> c_int {
    if out.is_null() {
        return remember(QuantionError::internal(
            FUNCTION,
            "the output buffer is null",
        ));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let options = read_options(
            matches,
            ppm_tolerance,
            mz_tolerance,
            rt_range,
            min_relative_intensity,
        )?;
        let features = read_features(mzs, rts, feature_count)?;
        let file = get_open_file(FUNCTION, file, "files")?;
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut file.source {
            let ranges = plan_sample_ranges(ion, &features, &options)
                .map_err(|error| from_fast_error(FUNCTION, error))?;
            super::public::write_ranges(out, &ranges);
        }
        Ok(())
    })
}

/// Plan the byte ranges `annotate_features` reads from the library, for every sample at once.
///
/// # Safety
/// `library` and every handle in `files` must be valid `ParsedFile`s; `model` a valid model.
/// `mzs` and `rts` must point to `feature_count` values. `out` must be a valid writable `Buf`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plan_annotate_library(
    library: *mut ParsedFile,
    model: *const ModelHandle,
    files: *const *mut ParsedFile,
    file_count: usize,
    mzs: *const f64,
    rts: *const f64,
    feature_count: usize,
    matches: usize,
    ppm_tolerance: f64,
    mz_tolerance: f64,
    rt_range: f64,
    min_relative_intensity: f64,
    out: *mut Buf,
) -> c_int {
    if out.is_null() {
        return remember(QuantionError::internal(
            FUNCTION,
            "the output buffer is null",
        ));
    }
    clear_buf(out);
    run(FUNCTION, || {
        let options = read_options(
            matches,
            ppm_tolerance,
            mz_tolerance,
            rt_range,
            min_relative_intensity,
        )?;
        let mut samples = read_files(library, files, file_count)?;
        let features = read_features(mzs, rts, feature_count)?;
        let model = read_model(model)?;
        let library = get_open_file(FUNCTION, library, "library")?;
        if let FileSource::Lazy(ion) | FileSource::Remote(ion) = &mut library.source {
            let ranges = plan_library_ranges(
                ion,
                &mut samples,
                &features,
                &options,
                model.vector_accession,
            )
            .map_err(|error| from_fast_error(FUNCTION, error))?;
            super::public::write_ranges(out, &ranges);
        }
        Ok(())
    })
}

fn get_number_or_nan(value: Option<f64>) -> f64 {
    value.unwrap_or(f64::NAN)
}

fn build_features_bridge(annotations: &[FeatureAnnotation]) -> Option<Box<[u8]>> {
    let numbers = |read: fn(&FeatureAnnotation) -> f64| -> Vec<f64> {
        annotations.iter().map(read).collect()
    };
    let mz = numbers(|annotation| annotation.mz);
    let rt = numbers(|annotation| annotation.rt);
    let frequency = numbers(|annotation| annotation.frequency);
    let score = numbers(|annotation| get_number_or_nan(annotation.score));
    let cosine = numbers(|annotation| get_number_or_nan(annotation.cosine));
    let labels: Vec<Option<&str>> = annotations
        .iter()
        .map(|annotation| annotation.label.as_deref())
        .collect();
    let ids: Vec<Option<&str>> = annotations
        .iter()
        .map(|annotation| annotation.id.as_deref())
        .collect();
    let databases: Vec<Option<&str>> = annotations
        .iter()
        .map(|annotation| annotation.database.as_deref())
        .collect();
    build_record_bridge(
        QUANTION_PAYLOAD_FEATURE_ANNOTATIONS,
        annotations.len() as u64,
        &[
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_ANNOTATION_MZ,
                values: &mz,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_ANNOTATION_RT,
                values: &rt,
            },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_STARTS,
                bytes_id: QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_BYTES,
                missing_id: QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_MISSING,
                values: &labels,
            },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_FEATURE_ANNOTATION_ID_STARTS,
                bytes_id: QUANTION_SECTION_FEATURE_ANNOTATION_ID_BYTES,
                missing_id: QUANTION_SECTION_FEATURE_ANNOTATION_ID_MISSING,
                values: &ids,
            },
            Column::OptionalText {
                starts_id: QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_STARTS,
                bytes_id: QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_BYTES,
                missing_id: QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_MISSING,
                values: &databases,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_ANNOTATION_FREQUENCY,
                values: &frequency,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_ANNOTATION_SCORE,
                values: &score,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FEATURE_ANNOTATION_COSINE,
                values: &cosine,
            },
        ],
    )
}

fn build_files_bridge(annotations: &[FeatureAnnotation]) -> Option<Box<[u8]>> {
    let mut features = Vec::new();
    let mut files = Vec::new();
    let mut scan_indexes = Vec::new();
    for (position, annotation) in annotations.iter().enumerate() {
        for file in &annotation.files {
            features.push(position as u32);
            files.push(file.file as u32);
            scan_indexes.push(file.scan_index.map_or(f64::NAN, |index| index as f64));
        }
    }
    build_record_bridge(
        QUANTION_PAYLOAD_FILE_ANNOTATIONS,
        features.len() as u64,
        &[
            Column::Counts {
                id: QUANTION_SECTION_FILE_ANNOTATION_FEATURE,
                values: &features,
            },
            Column::Counts {
                id: QUANTION_SECTION_FILE_ANNOTATION_FILE,
                values: &files,
            },
            Column::Numbers {
                id: QUANTION_SECTION_FILE_ANNOTATION_SCAN_INDEX,
                values: &scan_indexes,
            },
        ],
    )
}

fn build_matches_bridge(annotations: &[FeatureAnnotation]) -> Option<Box<[u8]>> {
    let mut features = Vec::new();
    let mut files = Vec::new();
    let mut found = Vec::new();
    for (position, annotation) in annotations.iter().enumerate() {
        for file in &annotation.files {
            for compound in &file.matches {
                features.push(position as u32);
                files.push(file.file as u32);
                found.push(compound);
            }
        }
    }
    let labels: Vec<&str> = found
        .iter()
        .map(|compound| compound.label.as_str())
        .collect();
    let ids: Vec<&str> = found.iter().map(|compound| compound.id.as_str()).collect();
    let databases: Vec<&str> = found
        .iter()
        .map(|compound| compound.database.as_str())
        .collect();
    let score: Vec<f64> = found.iter().map(|compound| compound.score).collect();
    let cosine: Vec<f64> = found.iter().map(|compound| compound.cosine).collect();
    let mz: Vec<&[f64]> = found
        .iter()
        .map(|compound| compound.mz.as_slice())
        .collect();
    let intensity: Vec<&[f64]> = found
        .iter()
        .map(|compound| compound.intensity.as_slice())
        .collect();
    let separator = FIELD_SEPARATOR.to_string();
    let names: Vec<String> = found
        .iter()
        .map(|compound| {
            compound
                .metadata
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(&separator)
        })
        .collect();
    let values: Vec<String> = found
        .iter()
        .map(|compound| {
            compound
                .metadata
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join(&separator)
        })
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let values: Vec<&str> = values.iter().map(String::as_str).collect();
    build_record_bridge(
        QUANTION_PAYLOAD_COMPOUND_MATCHES,
        found.len() as u64,
        &[
            Column::Counts {
                id: QUANTION_SECTION_COMPOUND_MATCH_FEATURE,
                values: &features,
            },
            Column::Counts {
                id: QUANTION_SECTION_COMPOUND_MATCH_FILE,
                values: &files,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_LABEL_STARTS,
                bytes_id: QUANTION_SECTION_COMPOUND_MATCH_LABEL_BYTES,
                values: &labels,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_ID_STARTS,
                bytes_id: QUANTION_SECTION_COMPOUND_MATCH_ID_BYTES,
                values: &ids,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_DATABASE_STARTS,
                bytes_id: QUANTION_SECTION_COMPOUND_MATCH_DATABASE_BYTES,
                values: &databases,
            },
            Column::Numbers {
                id: QUANTION_SECTION_COMPOUND_MATCH_SCORE,
                values: &score,
            },
            Column::Numbers {
                id: QUANTION_SECTION_COMPOUND_MATCH_COSINE,
                values: &cosine,
            },
            Column::NumberLists {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_MZ_STARTS,
                values_id: QUANTION_SECTION_COMPOUND_MATCH_MZ_VALUES,
                values: &mz,
            },
            Column::NumberLists {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_INTENSITY_STARTS,
                values_id: QUANTION_SECTION_COMPOUND_MATCH_INTENSITY_VALUES,
                values: &intensity,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_METADATA_NAME_STARTS,
                bytes_id: QUANTION_SECTION_COMPOUND_MATCH_METADATA_NAME_BYTES,
                values: &names,
            },
            Column::Text {
                starts_id: QUANTION_SECTION_COMPOUND_MATCH_METADATA_VALUE_STARTS,
                bytes_id: QUANTION_SECTION_COMPOUND_MATCH_METADATA_VALUE_BYTES,
                values: &values,
            },
        ],
    )
}
