use std::{cmp::Ordering, collections::BinaryHeap, sync::Arc};

use ionic::{
    ByteRangesOptions, IonReader, Range,
    source::{ByteRange, merge_ranges},
};

use crate::utilities::{
    calculate_eic::{FastError, TimeUnit, scan_in_minutes},
    ion::{ScanSource, ScanSummary},
    structs::{FromTo, Position, ScanQuery, SpectrumSummary},
};

#[derive(Clone, Debug)]
pub struct CentroidScan {
    pub index: usize,
    pub rt: f64,
    pub mz: Arc<[f64]>,
    pub intensity: Arc<[f64]>,
    pub metadata: SpectrumSummary,
}

pub struct ScanArrays {
    pub scan: CentroidScan,
    pub arrays: Vec<Vec<f64>>,
}

pub struct WantedScans {
    pub scans: Vec<(usize, ScanSummary)>,
    pub only_first: bool,
    rt: Position,
    mz: Position,
}

impl WantedScans {
    pub fn into_closest_first(self) -> ClosestScans {
        let scans = self.scans.into_iter().map(|(index, summary)| MatchingScan {
            mz_distance: get_distance(self.mz, summary.selected_ion_mz),
            rt_distance: get_distance(self.rt, summary.rt),
            index,
            summary,
        });
        ClosestScans {
            scans: scans.collect(),
        }
    }
}

struct MatchingScan {
    mz_distance: f64,
    rt_distance: f64,
    index: usize,
    summary: ScanSummary,
}

impl MatchingScan {
    fn compare(&self, other: &Self) -> Ordering {
        self.mz_distance
            .total_cmp(&other.mz_distance)
            .then(self.rt_distance.total_cmp(&other.rt_distance))
            .then(self.summary.rt.total_cmp(&other.summary.rt))
            .then(self.index.cmp(&other.index))
    }
}

impl Ord for MatchingScan {
    fn cmp(&self, other: &Self) -> Ordering {
        other.compare(self)
    }
}

impl PartialOrd for MatchingScan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MatchingScan {
    fn eq(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Equal
    }
}

impl Eq for MatchingScan {}

pub struct ClosestScans {
    scans: BinaryHeap<MatchingScan>,
}

impl Iterator for ClosestScans {
    type Item = (usize, ScanSummary);

    fn next(&mut self) -> Option<Self::Item> {
        self.scans
            .pop()
            .map(|closest| (closest.index, closest.summary))
    }
}

pub fn get_scans(
    source: &mut impl ScanSource,
    query: ScanQuery,
    time_unit: TimeUnit,
    level: u8,
) -> Vec<CentroidScan> {
    let wanted = find_wanted_scans(source, query, time_unit, level);
    if wanted.only_first {
        return load_first(source, wanted.into_closest_first());
    }
    load_selected(source, wanted.scans)
}

pub fn find_wanted_scans(
    source: &mut impl ScanSource,
    query: ScanQuery,
    time_unit: TimeUnit,
    level: u8,
) -> WantedScans {
    let rt = get_position_in_minutes(query.rt, time_unit);
    let mut scans: Vec<(usize, ScanSummary)> = Vec::new();
    source.for_each_summary(&mut |index, summary| {
        let summary = scan_in_minutes(summary);
        let is_level = level == 0 || summary.ms_level == level;
        if is_level && is_at(rt, summary.rt) && is_at(query.mz, summary.selected_ion_mz) {
            scans.push((index, summary));
        }
    });
    let only_first = matches!(rt, Position::Closest(_)) || matches!(query.mz, Position::Closest(_));
    if !only_first {
        scans.sort_by(|left, right| left.1.rt.total_cmp(&right.1.rt).then(left.0.cmp(&right.0)));
    }
    WantedScans {
        scans,
        only_first,
        rt,
        mz: query.mz,
    }
}

fn get_position_in_minutes(position: Position, time_unit: TimeUnit) -> Position {
    match position {
        Position::Any => Position::Any,
        Position::Range(range) => Position::Range(FromTo {
            from: time_unit.to_minutes(range.from.min(range.to)),
            to: time_unit.to_minutes(range.from.max(range.to)),
        }),
        Position::Closest(value) => Position::Closest(time_unit.to_minutes(value)),
    }
}

fn is_at(position: Position, value: f64) -> bool {
    match position {
        Position::Any => true,
        Position::Range(range) => {
            value.is_finite()
                && value >= range.from.min(range.to)
                && value <= range.from.max(range.to)
        }
        Position::Closest(_) => value.is_finite(),
    }
}

fn get_distance(position: Position, value: f64) -> f64 {
    match position {
        Position::Closest(target) => (value - target).abs(),
        Position::Any | Position::Range(_) => 0.0,
    }
}

pub fn get_scan_arrays(
    source: &mut impl ScanSource,
    query: ScanQuery,
    time_unit: TimeUnit,
    level: u8,
    accessions: &[u32],
) -> Vec<ScanArrays> {
    let wanted = find_wanted_scans(source, query, time_unit, level);
    if wanted.only_first {
        let first = wanted
            .into_closest_first()
            .find_map(|(index, summary)| read_arrays(source, index, summary, accessions));
        return first.into_iter().collect();
    }
    wanted
        .scans
        .into_iter()
        .filter_map(|(index, summary)| read_arrays(source, index, summary, accessions))
        .collect()
}

fn read_arrays(
    source: &mut impl ScanSource,
    index: usize,
    summary: ScanSummary,
    accessions: &[u32],
) -> Option<ScanArrays> {
    let mut arrays = Vec::with_capacity(accessions.len());
    let mut has_data = false;
    for &accession in accessions {
        let mut values = Vec::new();
        has_data |= source.load_array(index, accession, &mut values);
        arrays.push(values);
    }
    has_data.then(|| ScanArrays {
        scan: CentroidScan {
            index,
            rt: summary.rt,
            mz: Arc::from(&[][..]),
            intensity: Arc::from(&[][..]),
            metadata: SpectrumSummary::from_summary(&summary),
        },
        arrays,
    })
}

fn load_first(
    source: &mut impl ScanSource,
    candidates: impl Iterator<Item = (usize, ScanSummary)>,
) -> Vec<CentroidScan> {
    let mut buffers = ScanBuffers::default();
    for (index, summary) in candidates {
        if let Some(scan) = buffers.load(source, index, &summary) {
            return vec![scan];
        }
    }
    Vec::new()
}

fn load_selected(
    source: &mut impl ScanSource,
    candidates: Vec<(usize, ScanSummary)>,
) -> Vec<CentroidScan> {
    let mut buffers = ScanBuffers::default();
    candidates
        .into_iter()
        .filter_map(|(index, summary)| buffers.load(source, index, &summary))
        .collect()
}

#[derive(Default)]
struct ScanBuffers {
    mz: Vec<f64>,
    intensity: Vec<f64>,
}

impl ScanBuffers {
    fn load(
        &mut self,
        source: &mut impl ScanSource,
        index: usize,
        summary: &ScanSummary,
    ) -> Option<CentroidScan> {
        if !source.load_scan(index, &mut self.mz, &mut self.intensity) {
            return None;
        }
        let length = self.mz.len().min(self.intensity.len());
        Some(CentroidScan {
            index,
            rt: summary.rt,
            mz: Arc::from(&self.mz[..length]),
            intensity: Arc::from(&self.intensity[..length]),
            metadata: SpectrumSummary::from_summary(summary),
        })
    }
}

pub fn plan_scan_ranges(
    ion: &mut IonReader,
    query: ScanQuery,
    time_unit: TimeUnit,
    level: u8,
    arrays: &ByteRangesOptions,
) -> Result<Vec<ByteRange>, FastError> {
    ion.require_bounds().map_err(FastError::from)?;

    let wanted = find_wanted_scans(ion, query, time_unit, level);
    if wanted.only_first {
        return ranges_for_first_scan_with_data(ion, wanted.into_closest_first(), arrays);
    }
    ranges_for_scans(ion, &wanted.scans, arrays)
}

pub(crate) fn whole_scan_mz_range() -> Range {
    Range {
        from: 0.0,
        to: f64::MAX,
    }
}

fn ranges_for_scans(
    ion: &mut IonReader,
    scans: &[(usize, ScanSummary)],
    arrays: &ByteRangesOptions,
) -> Result<Vec<ByteRange>, FastError> {
    let mut ranges = Vec::with_capacity(scans.len());
    for (index, _) in scans {
        let scan_ranges = ion
            .byte_ranges(*index, whole_scan_mz_range(), arrays)
            .map_err(FastError::from)?;
        ranges.extend(scan_ranges);
    }
    merge_ranges(&mut ranges, 0);
    Ok(ranges)
}

fn ranges_for_first_scan_with_data(
    ion: &mut IonReader,
    scans: ClosestScans,
    arrays: &ByteRangesOptions,
) -> Result<Vec<ByteRange>, FastError> {
    for (index, _) in scans {
        let mut ranges = ion
            .byte_ranges(index, whole_scan_mz_range(), arrays)
            .map_err(FastError::from)?;
        if !ranges.is_empty() {
            merge_ranges(&mut ranges, 0);
            return Ok(ranges);
        }
    }
    Ok(Vec::new())
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_rts(scans: &[CentroidScan]) -> Vec<f64> {
        scans.iter().map(|scan| scan.rt).collect()
    }

    struct MockSource {
        scans: Vec<(ScanSummary, Vec<f64>, Vec<f64>)>,
    }

    impl MockSource {
        fn new(scans: Vec<(ScanSummary, Vec<f64>, Vec<f64>)>) -> Self {
            Self { scans }
        }
    }

    impl ScanSource for MockSource {
        fn for_each_summary(&mut self, cb: &mut dyn FnMut(usize, ScanSummary)) {
            for (i, (summary, _, _)) in self.scans.iter().enumerate() {
                cb(i, *summary);
            }
        }

        fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool {
            let Some((_, m, i)) = self.scans.get(index) else {
                return false;
            };
            *mz = m.clone();
            *intensity = i.clone();
            !m.is_empty()
        }

        fn load_array(&mut self, _index: usize, _accession: u32, values: &mut Vec<f64>) -> bool {
            values.clear();
            false
        }

        fn load_metadata(
            &mut self,
            index: usize,
        ) -> ionic::IonResult<Vec<crate::files::ScanParam>> {
            Err(ionic::IonError::OutOfRange {
                index,
                count: self.scans.len() as u64,
            })
        }
    }

    fn make_scan(
        rt: f64,
        ms_level: u8,
        selected_ion_mz: f64,
        mz: Vec<f64>,
        intensity: Vec<f64>,
    ) -> (ScanSummary, Vec<f64>, Vec<f64>) {
        (
            ScanSummary {
                rt,
                rt_unit: ionic::TimeUnit::Minute,
                ms_level,
                polarity: 0,
                selected_ion_mz,
                base_peak_mz: f64::NAN,
                base_peak_int: f64::NAN,
                total_ion_current: f64::NAN,
                position_x: 0,
                position_y: 0,
                position_z: 0,
            },
            mz,
            intensity,
        )
    }

    #[test]
    fn get_by_rt_range_returns_matching_scans() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![100.0], vec![10.0]),
            make_scan(2.0, 1, f64::NAN, vec![200.0], vec![20.0]),
            make_scan(3.0, 1, f64::NAN, vec![300.0], vec![30.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Range(FromTo { from: 1.0, to: 2.5 }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![1.0, 2.0]);
        assert_eq!(scans.len(), 2);
    }

    #[test]
    fn get_by_rt_range_filters_wrong_ms_level() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![100.0], vec![10.0]),
            make_scan(2.0, 2, f64::NAN, vec![200.0], vec![20.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Range(FromTo { from: 0.0, to: 5.0 }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![1.0]);
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn get_by_rt_range_empty_when_no_match() {
        let mut source =
            MockSource::new(vec![make_scan(5.0, 1, f64::NAN, vec![100.0], vec![10.0])]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Range(FromTo { from: 0.0, to: 2.0 }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert!(rts.is_empty());
        assert!(scans.is_empty());
    }

    #[test]
    fn get_closest_rt_picks_nearest_scan() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![100.0], vec![10.0]),
            make_scan(3.0, 1, f64::NAN, vec![300.0], vec![30.0]),
            make_scan(5.0, 1, f64::NAN, vec![500.0], vec![50.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Closest(2.8),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![3.0]);
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn get_closest_rt_falls_back_when_best_fails_to_load() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![], vec![]),
            make_scan(2.0, 1, f64::NAN, vec![200.0], vec![20.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Closest(1.0),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn get_closest_rt_empty_when_source_empty() {
        let mut source = MockSource::new(vec![]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Closest(1.0),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            1,
        );
        let rts = get_rts(&scans);
        assert!(rts.is_empty());
        assert!(scans.is_empty());
    }

    #[test]
    fn get_by_mz_range_returns_matching_scans() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 2, 500.0, vec![500.0], vec![100.0]),
            make_scan(2.0, 2, 600.0, vec![600.0], vec![200.0]),
            make_scan(3.0, 2, 700.0, vec![700.0], vec![300.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                mz: Position::Range(FromTo {
                    from: 490.0,
                    to: 650.0,
                }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            2,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts.len(), 2);
        assert!(rts.contains(&1.0) && rts.contains(&2.0));
        assert_eq!(scans.len(), 2);
    }

    #[test]
    fn get_by_mz_range_skips_nan_selected_ion_mz() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 2, f64::NAN, vec![100.0], vec![10.0]),
            make_scan(2.0, 2, 600.0, vec![600.0], vec![200.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                mz: Position::Range(FromTo {
                    from: 0.0,
                    to: 1000.0,
                }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            2,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn get_closest_mz_picks_nearest_selected_ion() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 2, 500.0, vec![500.0], vec![100.0]),
            make_scan(2.0, 2, 600.0, vec![600.0], vec![200.0]),
            make_scan(3.0, 2, 700.0, vec![700.0], vec![300.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                mz: Position::Closest(610.0),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            2,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
        assert_eq!(scans[0].metadata.selected_ion_mz, 600.0);
    }

    #[test]
    fn get_closest_mz_falls_back_when_best_fails_to_load() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 2, 500.0, vec![], vec![]),
            make_scan(2.0, 2, 510.0, vec![510.0], vec![200.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                mz: Position::Closest(500.0),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            2,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
        assert_eq!(scans.len(), 1);
    }

    #[test]
    fn get_closest_mz_empty_when_no_finite_selected_ion_mz() {
        let mut source =
            MockSource::new(vec![make_scan(1.0, 2, f64::NAN, vec![100.0], vec![10.0])]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                mz: Position::Closest(500.0),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            2,
        );
        let rts = get_rts(&scans);
        assert!(rts.is_empty());
        assert!(scans.is_empty());
    }

    #[test]
    fn ms_level_zero_allows_all_levels() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![100.0], vec![10.0]),
            make_scan(2.0, 2, f64::NAN, vec![200.0], vec![20.0]),
        ]);
        let scans = get_scans(
            &mut source,
            ScanQuery {
                rt: Position::Range(FromTo { from: 0.0, to: 5.0 }),
                ..Default::default()
            },
            TimeUnit::MINUTES,
            0,
        );
        let rts = get_rts(&scans);
        assert_eq!(rts.len(), 2);
        assert_eq!(scans.len(), 2);
    }

    fn make_mixed_source() -> MockSource {
        MockSource::new(vec![
            make_scan(1.0, 2, 300.0, vec![1.0], vec![1.0]),
            make_scan(2.0, 2, 300.0, vec![2.0], vec![1.0]),
            make_scan(2.1, 2, 400.0, vec![3.0], vec![1.0]),
            make_scan(5.0, 2, 300.0, vec![4.0], vec![1.0]),
        ])
    }

    #[test]
    fn rt_range_and_mz_range_keep_scans_inside_both() {
        let query = ScanQuery {
            rt: Position::Range(FromTo { from: 1.5, to: 3.0 }),
            mz: Position::Range(FromTo {
                from: 299.0,
                to: 301.0,
            }),
        };
        let scans = get_scans(&mut make_mixed_source(), query, TimeUnit::MINUTES, 2);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
    }

    #[test]
    fn rt_range_and_closest_mz_pick_one_scan_inside_the_range() {
        let query = ScanQuery {
            rt: Position::Range(FromTo { from: 1.5, to: 3.0 }),
            mz: Position::Closest(390.0),
        };
        let scans = get_scans(&mut make_mixed_source(), query, TimeUnit::MINUTES, 2);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.1]);
    }

    #[test]
    fn closest_rt_and_mz_range_pick_the_nearest_scan_at_that_precursor() {
        let query = ScanQuery {
            rt: Position::Closest(4.0),
            mz: Position::Range(FromTo {
                from: 299.0,
                to: 301.0,
            }),
        };
        let scans = get_scans(&mut make_mixed_source(), query, TimeUnit::MINUTES, 2);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![5.0]);
    }

    #[test]
    fn closest_rt_and_closest_mz_pick_the_closest_precursor_then_the_closest_rt() {
        let query = ScanQuery {
            rt: Position::Closest(2.1),
            mz: Position::Closest(300.0),
        };
        let scans = get_scans(&mut make_mixed_source(), query, TimeUnit::MINUTES, 2);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
    }

    #[test]
    fn rt_range_in_seconds_is_read_in_minutes() {
        let query = ScanQuery {
            rt: Position::Range(FromTo {
                from: 90.0,
                to: 180.0,
            }),
            mz: Position::Closest(300.0),
        };
        let scans = get_scans(&mut make_mixed_source(), query, TimeUnit::SECONDS, 2);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![2.0]);
    }

    #[test]
    fn closest_skips_empty_scans_in_closest_first_order() {
        let mut source = MockSource::new(vec![
            make_scan(1.0, 1, f64::NAN, vec![], vec![]),
            make_scan(3.0, 1, f64::NAN, vec![300.0], vec![30.0]),
            make_scan(1.1, 1, f64::NAN, vec![], vec![]),
            make_scan(1.2, 1, f64::NAN, vec![120.0], vec![12.0]),
        ]);
        let query = ScanQuery {
            rt: Position::Closest(1.0),
            ..Default::default()
        };
        let scans = get_scans(&mut source, query, TimeUnit::MINUTES, 1);
        let rts = get_rts(&scans);
        assert_eq!(rts, vec![1.2]);
    }
}
