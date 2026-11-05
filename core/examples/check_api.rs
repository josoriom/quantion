use std::path::Path;

use quantion::{
    DataXY, CalculateEicOptions, DeisotopeFeaturesOptions, Feature, FindFeaturesOptions, FindPeaksOptions, FindTargetsOptions, FindTransitionsOptions,
    GetPeakOptions, GetScanMetadataOptions, GridOptions, MatchFeaturesOptions, ParseIonOptions, Peak, SampleFile, ScanQuery, Target,
    Transition, FromTo, calculate_eic, deisotope_features, find_features, find_noise_level, find_peaks, find_targets, find_transitions,
    get_peak, get_scan_metadata, get_scans, load_model, load_warp, match_features, parse_ion,
};

const FROM_RT: f64 = 0.0;
const TO_RT: f64 = 5.0;
const EIC_PPM: f64 = 20.0;
const EIC_MZ: f64 = 0.005;
const GRID_STEP: f64 = 0.005;
const GRID_START: f64 = 89.0;
const GRID_END: f64 = 91.0;
const ROI_RANGE: f64 = 0.05;
const THREADS: usize = 1;

const FEATURE_MIN_INTENSITY: f64 = 500.0;
const FEATURE_MIN_PEAK_WIDTH_POINTS: usize = 5;

const SELF_CHECK_VALUES: [f64; 3] = [1.0, 2.0, 3.0];

struct Channel {
    name: &'static str,
    mass: f64,
    min_intensity: f64,
    min_peak_width_points: usize,
    targets: [(&'static str, f64); 3],
}

const CHANNELS: [Channel; 2] = [
    Channel {
        name: "mz_2",
        mass: 90.05550,
        min_intensity: 500.0,
        min_peak_width_points: 5,
        targets: [("met_1", 2.885), ("met_2", 2.552), ("met_3", 2.465)],
    },
    Channel {
        name: "mz_1",
        mass: 89.04768,
        min_intensity: 500.0,
        min_peak_width_points: 5,
        targets: [("met_1", 2.385), ("met_2", 2.18), ("met_3", 2.08)],
    },
];

fn number(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }
    if value.is_infinite() {
        return if value > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    format!("{value}")
}

fn digest(values: &[f64]) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for value in values {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= byte as u32;
            hash = hash.wrapping_mul(0x0100_0193);
        }
    }
    format!("{hash:08x}")
}

fn eic_options() -> CalculateEicOptions {
    CalculateEicOptions {
        from: FROM_RT,
        to: TO_RT,
        ppm_tolerance: EIC_PPM,
        mz_tolerance: EIC_MZ,
        ..Default::default()
    }
}

fn peak_options(channel: &Channel) -> FindPeaksOptions {
    FindPeaksOptions {
        min_intensity: channel.min_intensity,
        min_peak_width_points: channel.min_peak_width_points,
        auto_noise: true,
        auto_baseline: true,
        ..Default::default()
    }
}

fn chrom_peak_options(channel: &Channel) -> FindPeaksOptions {
    FindPeaksOptions {
        auto_noise: false,
        auto_baseline: false,
        ..peak_options(channel)
    }
}

fn feature_peak_options() -> FindPeaksOptions {
    FindPeaksOptions {
        min_intensity: FEATURE_MIN_INTENSITY,
        min_peak_width_points: FEATURE_MIN_PEAK_WIDTH_POINTS,
        auto_noise: true,
        auto_baseline: true,
        ..Default::default()
    }
}

fn grid_options() -> GridOptions {
    GridOptions {
        start: GRID_START,
        end: GRID_END,
        step_size: GRID_STEP,
    }
}

fn open(fixture: &Path) -> SampleFile {
    parse_ion(fixture.to_path_buf(), ParseIonOptions::default()).expect("open fixture")
}

fn count_scans(fixture: &Path) -> usize {
    let query = ScanQuery::RtRange(FromTo { from: FROM_RT, to: TO_RT });
    get_scans(&mut open(fixture), query, Default::default()).expect("get_scans").len()
}

fn scan_metadata(fixture: &Path, out: &mut Vec<String>) {
    let mut file = open(fixture);
    let query = ScanQuery::RtRange(FromTo { from: FROM_RT, to: TO_RT });
    let scans = get_scans(&mut file, query, Default::default()).expect("get_scans");
    let first = scans.first().expect("a first scan").index;
    let last = scans.last().expect("a last scan").index;
    out.push(format!("get_scans.first.index = {first}"));
    out.push(format!("get_scans.last.index = {last}"));
    let rows = get_scan_metadata(&mut file, last, Default::default()).expect("get_scan_metadata");
    out.push(format!("get_scan_metadata.count = {}", rows.len()));
    for (position, row) in rows.iter().enumerate() {
        let kind = format!("{:?}", row.kind);
        let fields = [Some(row.path.as_str()), Some(kind.as_str()), row.accession.as_deref(), Some(row.name.as_str()), row.value.as_deref(), row.unit.as_deref(), row.unit_accession.as_deref()];
        let text: Vec<&str> = fields.iter().map(|field| field.unwrap_or("NA")).collect();
        out.push(format!("get_scan_metadata.{position} = {}", text.join(" | ")));
    }
    let options = GetScanMetadataOptions { tags: vec!["scan".to_string()] };
    let scan_rows = get_scan_metadata(&mut file, last, options).expect("get_scan_metadata");
    out.push(format!("get_scan_metadata.scan.count = {}", scan_rows.len()));
}

fn extract_eic(fixture: &Path, channel: &Channel) -> DataXY {
    calculate_eic(&mut open(fixture), channel.mass, eic_options()).expect("calculate_eic")
}

fn peak_lines(prefix: &str, peak: &Peak, out: &mut Vec<String>) {
    shared_peak_lines(prefix, peak, out);
    out.push(format!("{prefix}.n_points = {}", peak.n_points));
}

fn shared_peak_lines(prefix: &str, peak: &Peak, out: &mut Vec<String>) {
    out.push(format!("{prefix}.rt = {}", number(peak.rt)));
    out.push(format!("{prefix}.from = {}", number(peak.from)));
    out.push(format!("{prefix}.to = {}", number(peak.to)));
    out.push(format!("{prefix}.integral = {}", number(peak.integral)));
    out.push(format!("{prefix}.intensity = {}", number(peak.intensity)));
}

fn report_targets(fixture: &Path, channel: &Channel, prefix: &str, out: &mut Vec<String>) {
    let targets: Vec<Target> = channel
        .targets
        .iter()
        .map(|(id, rt)| Target {
            id: Some(id.to_string()),
            mz: channel.mass,
            rt: *rt,
            range: Some(ROI_RANGE),
        })
        .collect();
    let options = FindTargetsOptions {
        eic: eic_options(),
        find_peaks: peak_options(channel),
        threads: THREADS,
    };
    let rows = find_targets(&mut open(fixture), &targets, options).expect("find_targets");
    for ((id, _), peak) in channel.targets.iter().zip(&rows) {
        shared_peak_lines(&format!("{prefix}.{id}"), peak, out);
    }
}

fn report_transitions(fixture: &Path, channel: &Channel, prefix: &str, out: &mut Vec<String>) {
    let folder = fixture.parent().and_then(Path::parent).expect("fixtures folder");
    let targets: Vec<Transition> = channel
        .targets
        .iter()
        .map(|(id, rt)| Transition {
            id: Some(id.to_string()),
            precursor: channel.mass,
            product: channel.mass,
            rt: *rt,
            range: ROI_RANGE,
        })
        .collect();
    let options = FindTransitionsOptions {
        find_peaks: chrom_peak_options(channel),
        threads: THREADS,
    };
    let rows = find_transitions(&mut open(&folder.join("srm.ion")), &targets, options).expect("find_transitions");
    for ((id, _), peak) in channel.targets.iter().zip(&rows) {
        let name = format!("{prefix}.{id}");
        out.push(format!("{name}.rt = {}", number(peak.rt)));
        out.push(format!("{name}.from = {}", number(peak.from)));
        out.push(format!("{name}.to = {}", number(peak.to)));
        out.push(format!("{name}.intensity = {}", number(peak.intensity)));
        out.push(format!("{name}.area = {}", number(peak.integral)));
    }
}

fn features(fixture: &Path, out: &mut Vec<String>) {
    let options = FindFeaturesOptions {
        eic: eic_options(),
        grid: grid_options(),
        find_peaks: feature_peak_options(),
        threads: THREADS,
    };
    let found = find_features(&mut open(fixture), options).expect("find_features");
    out.push(format!("find_features.count = {}", found.len()));
    for (index, f) in found.iter().enumerate() {
        let name = format!("find_features[{index}]");
        out.push(format!("{name}.mz = {}", number(f.mz)));
        out.push(format!("{name}.rt = {}", number(f.rt)));
        out.push(format!("{name}.from = {}", number(f.from)));
        out.push(format!("{name}.to = {}", number(f.to)));
        out.push(format!("{name}.integral = {}", number(f.integral)));
        out.push(format!("{name}.intensity = {}", number(f.intensity)));
        out.push(format!("{name}.n_points = {}", f.n_points));
    }
}

fn consensus_features(directory: &str, out: &mut Vec<String>) {
    let options = MatchFeaturesOptions {
        eic: eic_options(),
        grid: grid_options(),
        find_peaks: feature_peak_options(),
        threads: THREADS,
        ..Default::default()
    };
    let found = match_features(directory, options).expect("match_features");
    report_matched("match_features", &found, out);
    let cleaned = deisotope_features(found, DeisotopeFeaturesOptions::default()).expect("deisotope_features");
    report_matched("deisotope_features", &cleaned, out);
}

fn report_matched(label: &str, found: &[Feature], out: &mut Vec<String>) {
    out.push(format!("{label}.count = {}", found.len()));
    for (index, f) in found.iter().enumerate() {
        let name = format!("{label}[{index}]");
        out.push(format!("{name}.mz = {}", number(f.mz)));
        out.push(format!("{name}.rt = {}", number(f.rt)));
        out.push(format!("{name}.from = {}", number(f.from)));
        out.push(format!("{name}.to = {}", number(f.to)));
        out.push(format!("{name}.integral = {}", number(f.integral)));
        out.push(format!("{name}.intensity = {}", number(f.intensity)));
    }
}

fn report_channel(fixture: &Path, channel: &Channel, out: &mut Vec<String>) {
    let name = channel.name;
    let eic = extract_eic(fixture, channel);
    let noise = find_noise_level(eic.y.as_slice());
    let data = DataXY {
        x: eic.x.clone(),
        y: eic.y.clone(),
    };

    out.push(format!("{name}.calculate_eic.points = {}", eic.x.len()));
    out.push(format!("{name}.calculate_eic.x.fnv1a = {}", digest(&eic.x)));
    out.push(format!("{name}.calculate_eic.y.fnv1a = {}", digest(&eic.y)));
    out.push(format!(
        "{name}.find_noise_level.intensity = {}",
        number(noise.intensity)
    ));
    out.push(format!("{name}.find_noise_level.width = {}", noise.width));

    let found = find_peaks(&data, peak_options(channel)).expect("find_peaks");
    out.push(format!("{name}.find_peaks.count = {}", found.len()));
    for (index, peak) in found.iter().enumerate() {
        peak_lines(&format!("{name}.find_peaks[{index}]"), peak, out);
    }

    let target = channel.targets[2].1;
    let options = GetPeakOptions {
        range: ROI_RANGE,
        find_peaks: peak_options(channel),
    };
    let single = get_peak(&data, target, options).expect("get_peak");
    peak_lines(&format!("{name}.get_peak"), &single, out);

    report_targets(fixture, channel, &format!("{name}.find_targets"), out);
    report_transitions(fixture, channel, &format!("{name}.find_transitions"), out);
}

fn predicted_rts(fixture: &Path, directory: &str, out: &mut Vec<String>) {
    let model = Path::new(directory).join("..").join("shift_model.onnx");
    let warp = load_warp(&load_model(model.as_path()).expect("model")).expect("warp model");
    let mut file = open(fixture);
    let names: Vec<String> = CHANNELS
        .iter()
        .flat_map(|channel| {
            channel
                .targets
                .iter()
                .map(move |(id, _)| format!("{}.{id}", channel.name))
        })
        .collect();
    let targets: Vec<Target> = CHANNELS
        .iter()
        .flat_map(|channel| {
            channel.targets.iter().map(move |(id, rt)| Target {
                id: Some(id.to_string()),
                mz: channel.mass,
                rt: *rt,
                range: None,
            })
        })
        .collect();
    let predicted = warp
        .predict_rts(&mut file, &targets)
        .expect("predict_rts");
    out.push(format!("predict_rts.count = {}", predicted.len()));
    for (name, prediction) in names.iter().zip(&predicted) {
        out.push(format!("predict_rts.{name}.rt = {}", number(prediction.rt)));
        out.push(format!(
            "predict_rts.{name}.uncertainty = {}",
            number(prediction.uncertainty)
        ));
    }
}

fn main() {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "core/tests/fixtures/api".to_string());
    let fixture = Path::new(&directory).join("api.ion");

    let mut out = Vec::new();
    out.push(format!("fnv1a.self_check = {}", digest(&SELF_CHECK_VALUES)));
    out.push(format!("parse_ion.scans = {}", count_scans(&fixture)));
    scan_metadata(&fixture, &mut out);
    for channel in CHANNELS.iter() {
        report_channel(&fixture, channel, &mut out);
    }
    features(&fixture, &mut out);
    consensus_features(&directory, &mut out);
    predicted_rts(&fixture, &directory, &mut out);

    println!("{}", out.join("\n"));
}
