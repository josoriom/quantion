use std::fmt;

use runnet::Tensor;

use crate::{
    files::read_accession_number,
    utilities::{clean_spectrum::Spectrum, load_model::Model},
};

const SPECTRA_PER_RUN: usize = 256;
const PEAK_VALUES: usize = 2;
const MAX_PEAKS_KEY: &str = "max_peaks";
const MAX_MZ_KEY: &str = "max_mz";
const INTENSITY_SCALE_KEY: &str = "intensity_scale";
const VECTOR_ACCESSION_KEY: &str = "vector_accession";
const COMPOUND_KEY: &str = "compound";
const ID_KEY: &str = "id";
const DATABASE_KEY: &str = "database";
const LABEL_KEY: &str = "label";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SpectrumModelError {
    MissingKey(&'static str),
    InvalidKey {
        key: &'static str,
        rule: &'static str,
        value: String,
    },
    NotRun(String),
    WrongOutput,
}

impl fmt::Display for SpectrumModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey(key) => write!(formatter, "must have the metadata key {key}"),
            Self::InvalidKey { key, rule, value } => write!(
                formatter,
                "metadata key {key} must be {rule}, got {value:?}"
            ),
            Self::NotRun(reason) => write!(formatter, "could not be run: {reason}"),
            Self::WrongOutput => write!(formatter, "must return one vector per spectrum"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub(crate) enum IntensityScale {
    SQRT,
    NONE,
}

impl IntensityScale {
    fn apply(self, intensity: f64) -> f64 {
        match self {
            Self::SQRT => intensity.sqrt(),
            Self::NONE => intensity,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SpectrumModel {
    model: Model,
    pub max_peaks: usize,
    pub max_mz: f64,
    pub intensity_scale: IntensityScale,
    pub vector_accession: u32,
    pub compound_param: String,
    pub id_param: String,
    pub database_param: String,
    pub label_param: String,
}

impl SpectrumModel {
    pub(crate) fn read(model: &Model) -> Result<Self, SpectrumModelError> {
        let get_value = |key: &'static str| {
            model
                .get_value(key)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(SpectrumModelError::MissingKey(key))
        };
        let invalid =
            |key: &'static str, rule: &'static str, value: &str| SpectrumModelError::InvalidKey {
                key,
                rule,
                value: value.to_string(),
            };
        let max_peaks_text = get_value(MAX_PEAKS_KEY)?;
        let max_peaks = max_peaks_text
            .parse::<usize>()
            .ok()
            .filter(|count| *count > 0)
            .ok_or_else(|| invalid(MAX_PEAKS_KEY, "a whole number above 0", max_peaks_text))?;
        let max_mz_text = get_value(MAX_MZ_KEY)?;
        let max_mz = max_mz_text
            .parse::<f64>()
            .ok()
            .filter(|mz| mz.is_finite() && *mz > 0.0)
            .ok_or_else(|| invalid(MAX_MZ_KEY, "a number above 0", max_mz_text))?;
        let intensity_scale = match get_value(INTENSITY_SCALE_KEY)? {
            "SQRT" => IntensityScale::SQRT,
            "NONE" => IntensityScale::NONE,
            other => return Err(invalid(INTENSITY_SCALE_KEY, "SQRT or NONE", other)),
        };
        let accession_text = get_value(VECTOR_ACCESSION_KEY)?;
        let vector_accession = read_accession_number(accession_text).ok_or_else(|| {
            invalid(
                VECTOR_ACCESSION_KEY,
                "an accession like MS:1000786",
                accession_text,
            )
        })?;
        let compound_param = get_value(COMPOUND_KEY)?.to_string();
        let id_param = get_value(ID_KEY)?.to_string();
        let database_param = get_value(DATABASE_KEY)?.to_string();
        let label_param = get_value(LABEL_KEY)?.to_string();
        Ok(Self {
            model: model.clone(),
            max_peaks,
            max_mz,
            intensity_scale,
            vector_accession,
            compound_param,
            id_param,
            database_param,
            label_param,
        })
    }

    pub(crate) fn get_vectors(
        &self,
        spectra: &[Spectrum],
    ) -> Result<Vec<Vec<f64>>, SpectrumModelError> {
        let peaks: Vec<Spectrum> = spectra
            .iter()
            .map(|spectrum| self.get_model_peaks(spectrum))
            .collect();
        let mut order: Vec<usize> = (0..peaks.len()).collect();
        order.sort_by_key(|position| peaks[*position].mz.len());
        let mut vectors = vec![Vec::new(); peaks.len()];
        for group in order.chunks(SPECTRA_PER_RUN) {
            let group_peaks: Vec<&Spectrum> =
                group.iter().map(|position| &peaks[*position]).collect();
            for (position, vector) in group.iter().zip(self.run(&group_peaks)?) {
                vectors[*position] = vector;
            }
        }
        Ok(vectors)
    }

    fn run(&self, spectra: &[&Spectrum]) -> Result<Vec<Vec<f64>>, SpectrumModelError> {
        let width = spectra
            .iter()
            .map(|spectrum| spectrum.mz.len())
            .max()
            .unwrap_or(0)
            .max(1);
        let mut values = vec![0.0f32; spectra.len() * width * PEAK_VALUES];
        for (row, spectrum) in spectra.iter().enumerate() {
            for (column, (mz, intensity)) in spectrum.mz.iter().zip(&spectrum.intensity).enumerate()
            {
                let start = (row * width + column) * PEAK_VALUES;
                values[start] = *mz as f32;
                values[start + 1] = *intensity as f32;
            }
        }
        let outputs = self
            .model
            .run(vec![Tensor::from_floats(
                [spectra.len(), width, PEAK_VALUES],
                values,
            )])
            .map_err(|error| SpectrumModelError::NotRun(error.to_string()))?;
        let output = outputs
            .first()
            .and_then(|output| output.floats().ok())
            .ok_or(SpectrumModelError::WrongOutput)?;
        if output.is_empty() || output.len() % spectra.len() != 0 {
            return Err(SpectrumModelError::WrongOutput);
        }
        let size = output.len() / spectra.len();
        Ok(output
            .chunks(size)
            .map(|values| values.iter().map(|value| f64::from(*value)).collect())
            .collect())
    }

    fn get_model_peaks(&self, spectrum: &Spectrum) -> Spectrum {
        let mut peaks: Vec<usize> = (0..spectrum.mz.len())
            .filter(|position| {
                let (mz, intensity) = (spectrum.mz[*position], spectrum.intensity[*position]);
                mz > 0.0 && mz < self.max_mz && intensity > 0.0
            })
            .collect();
        if peaks.len() > self.max_peaks {
            peaks.sort_by(|left, right| {
                spectrum.intensity[*right].total_cmp(&spectrum.intensity[*left])
            });
            peaks.truncate(self.max_peaks);
        }
        let scaled: Vec<f64> = peaks
            .iter()
            .map(|position| self.intensity_scale.apply(spectrum.intensity[*position]))
            .collect();
        let length = scaled.iter().map(|value| value * value).sum::<f64>().sqrt();
        Spectrum {
            mz: peaks
                .iter()
                .map(|position| spectrum.mz[*position])
                .collect(),
            intensity: scaled
                .iter()
                .map(|value| if length > 0.0 { value / length } else { *value })
                .collect(),
        }
    }
}
