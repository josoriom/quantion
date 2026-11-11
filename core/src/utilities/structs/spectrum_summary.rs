use crate::utilities::ion::ScanSummary;

#[derive(Clone, Debug, Default)]
pub struct SpectrumSummary {
    pub rt_seconds: f64,
    pub base_peak_mz: f64,
    pub selected_ion_mz: f64,
    pub base_peak_int: f64,
    pub total_ion_current: f64,
    pub ms_level: u8,
    pub polarity: u8,
    pub position_x: u32,
    pub position_y: u32,
    pub position_z: u32,
}

impl SpectrumSummary {
    #[inline]
    pub fn unknown() -> Self {
        Self {
            rt_seconds: f64::NAN,
            base_peak_mz: f64::NAN,
            selected_ion_mz: f64::NAN,
            base_peak_int: f64::NAN,
            total_ion_current: f64::NAN,
            ms_level: 0,
            polarity: 0,
            position_x: 0,
            position_y: 0,
            position_z: 0,
        }
    }

    #[inline]
    pub fn from_summary(s: &ScanSummary) -> Self {
        Self {
            rt_seconds: s.rt * 60.0,
            base_peak_mz: s.base_peak_mz,
            selected_ion_mz: s.selected_ion_mz,
            base_peak_int: s.base_peak_int,
            total_ion_current: s.total_ion_current,
            ms_level: s.ms_level,
            polarity: s.polarity,
            position_x: s.position_x,
            position_y: s.position_y,
            position_z: s.position_z,
        }
    }
}
