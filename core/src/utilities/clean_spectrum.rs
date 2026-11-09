use crate::utilities::find_noise_level::find_noise_level;

pub(crate) const PEAK_TOLERANCE: f64 = 0.02;

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Spectrum {
    pub mz: Vec<f64>,
    pub intensity: Vec<f64>,
}

impl Spectrum {
    pub(crate) fn sort_by_mz(&mut self) {
        let mut pairs: Vec<(f64, f64)> = self
            .mz
            .iter()
            .copied()
            .zip(self.intensity.iter().copied())
            .collect();
        pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
        self.mz = pairs.iter().map(|pair| pair.0).collect();
        self.intensity = pairs.iter().map(|pair| pair.1).collect();
    }

    pub(crate) fn keep_below(&mut self, limit: f64) {
        let count = self.mz.iter().take_while(|mz| **mz < limit).count();
        self.mz.truncate(count);
        self.intensity.truncate(count);
    }
}

pub(crate) fn clean_spectrum(spectrum: Spectrum, min_relative_intensity: f64) -> Spectrum {
    let (peaks, keep) = if is_profile(&spectrum.intensity) {
        let noise = find_noise_level(spectrum.intensity.as_slice()).intensity;
        let peaks = merge_points_into_peaks(&spectrum);
        let keep: Vec<bool> = peaks
            .intensity
            .iter()
            .map(|intensity| *intensity > noise)
            .collect();
        (peaks, keep)
    } else {
        let limit = min_relative_intensity
            * spectrum
                .intensity
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
        let keep: Vec<bool> = spectrum
            .intensity
            .iter()
            .map(|intensity| *intensity >= limit)
            .collect();
        (spectrum, keep)
    };
    if !keep.contains(&true) {
        return peaks;
    }
    let mut kept = Spectrum::default();
    for ((mz, intensity), keep) in peaks.mz.iter().zip(&peaks.intensity).zip(keep) {
        if keep {
            kept.mz.push(*mz);
            kept.intensity.push(*intensity);
        }
    }
    kept
}

fn is_profile(intensity: &[f64]) -> bool {
    intensity.contains(&0.0)
}

fn merge_points_into_peaks(spectrum: &Spectrum) -> Spectrum {
    let mut peaks = Spectrum::default();
    let mut weighted_mz = 0.0;
    let mut total = 0.0;
    let mut last_mz = f64::NAN;
    for (mz, intensity) in spectrum.mz.iter().zip(&spectrum.intensity) {
        if *intensity <= 0.0 {
            continue;
        }
        if total > 0.0 && mz - last_mz > PEAK_TOLERANCE {
            peaks.mz.push(weighted_mz / total);
            peaks.intensity.push(total);
            weighted_mz = 0.0;
            total = 0.0;
        }
        weighted_mz += intensity * mz;
        total += intensity;
        last_mz = *mz;
    }
    if total > 0.0 {
        peaks.mz.push(weighted_mz / total);
        peaks.intensity.push(total);
    }
    peaks
}
