use ionic::{IonReader, source::ByteRange};

use crate::{
    files::CalculateEicOptions,
    targets::FindTargetsOptions,
    traces::FindPeaksOptions,
    utilities::{
        calculate_eic::{
            EicReader, FastError, MS1_LEVEL, get_scan_times, lower_bound, mz_tolerance_for,
            plan_window_ranges, read_mz_window, upper_bound,
        },
        get_peak::get_peak,
        structs::{DataXY, FromTo, Peak, Roi, Target},
    },
};

pub fn plan_targets_ranges(
    ion: &mut IonReader,
    targets: &[Target],
    from: f64,
    to: f64,
    options: CalculateEicOptions,
) -> Result<Vec<ByteRange>, FastError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let mut mz_from = f64::INFINITY;
    let mut mz_to = f64::NEG_INFINITY;
    for target in targets {
        let tolerance = mz_tolerance_for(target.mz, options);
        mz_from = mz_from.min(target.mz - tolerance);
        mz_to = mz_to.max(target.mz + tolerance);
    }

    plan_window_ranges(ion, from, to, mz_from, mz_to)
}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
use rayon::prelude::*;

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
use crate::utilities::parallel::run_with_threads;

pub(crate) struct MzWindow {
    pub(crate) from: f64,
    pub(crate) to: f64,
}

struct PeakJob {
    roi_idx: usize,
    rt: f64,
    range: f64,
    mz_from: f64,
    mz_to: f64,
}

pub fn find_targets(
    reader: &mut EicReader,
    targets: &[Target],
    window: FromTo,
    options: FindTargetsOptions,
) -> Result<Vec<Peak>, FastError> {
    if targets.is_empty() {
        return Ok(Vec::new());
    }

    let scan_times = get_scan_times(reader, window.from, window.to, MS1_LEVEL);

    if scan_times.len() < 3 {
        return Ok(vec![Peak::default(); targets.len()]);
    }

    let rts_full: Vec<f64> = scan_times.iter().map(|s| s.rt).collect();
    let scan_count = scan_times.len();

    let jobs = build_peak_jobs(targets, options.eic);
    if jobs.is_empty() {
        return Ok(vec![Peak::default(); targets.len()]);
    }

    let windows: Vec<MzWindow> = jobs
        .iter()
        .map(|job| MzWindow {
            from: job.mz_from,
            to: job.mz_to,
        })
        .collect();
    let eic_values = read_eics_one_pass(reader, &scan_times, &windows)?;

    let peaks = compute_peaks_for_jobs(
        &jobs,
        &eic_values,
        scan_count,
        &rts_full,
        &options.find_peaks,
        window,
        options.threads,
    );

    let mut results = vec![Peak::default(); targets.len()];
    for (roi_idx, peak) in peaks {
        results[roi_idx] = peak;
    }

    Ok(results)
}

fn build_peak_jobs(targets: &[Target], eic_opts: CalculateEicOptions) -> Vec<PeakJob> {
    targets
        .iter()
        .enumerate()
        .map(|(idx, target)| {
            let tolerance = mz_tolerance_for(target.mz, eic_opts);
            PeakJob {
                roi_idx: idx,
                rt: target.rt,
                range: target.get_range(),
                mz_from: target.mz - tolerance,
                mz_to: target.mz + tolerance,
            }
        })
        .collect()
}

fn compute_peak_for_job(
    job: &PeakJob,
    y_full: &[f64],
    rts_full: &[f64],
    options: &FindPeaksOptions,
    window: FromTo,
) -> Peak {
    const HALF_WIDTH: f64 = 0.5;
    let local_from = (job.rt - HALF_WIDTH).max(window.from);
    let local_to = (job.rt + HALF_WIDTH).min(window.to);
    if local_to <= local_from {
        return Peak::default();
    }

    let start = lower_bound(rts_full, local_from);
    let end = upper_bound(rts_full, local_to).min(rts_full.len());
    if end.saturating_sub(start) < 3 {
        return Peak::default();
    }

    get_peak(
        &DataXY {
            x: rts_full.to_vec(),
            y: y_full.to_vec(),
        },
        &Roi {
            rt: job.rt,
            range: job.range,
        },
        options,
    )
}

pub(crate) fn read_eics_one_pass(
    reader: &mut EicReader,
    scan_times: &[crate::utilities::calculate_eic::ScanTime],
    windows: &[MzWindow],
) -> Result<Vec<f64>, FastError> {
    let scan_count = scan_times.len();
    let mut eic_values = vec![0.0; windows.len() * scan_count];

    let mz_from = windows
        .iter()
        .map(|window| window.from)
        .fold(f64::INFINITY, f64::min);
    let mz_to = windows
        .iter()
        .map(|window| window.to)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut mz_values = Vec::new();
    let mut intensity_values = Vec::new();

    for (scan_position, scan_time) in scan_times.iter().enumerate() {
        read_mz_window(
            reader,
            scan_time.index,
            mz_from,
            mz_to,
            &mut mz_values,
            &mut intensity_values,
        )?;

        for (window_index, window) in windows.iter().enumerate() {
            let start = lower_bound(&mz_values, window.from);
            let end = upper_bound(&mz_values, window.to).min(mz_values.len());
            eic_values[window_index * scan_count + scan_position] =
                intensity_values[start..end].iter().sum();
        }
    }

    Ok(eic_values)
}

fn compute_peaks_for_jobs(
    jobs: &[PeakJob],
    eic_values: &[f64],
    scan_count: usize,
    rts_full: &[f64],
    options: &FindPeaksOptions,
    window: FromTo,
    threads: usize,
) -> Vec<(usize, Peak)> {
    #[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
    if threads > 1 && jobs.len() > 1 {
        return run_with_threads(threads, || {
            compute_peaks_for_jobs_parallel(jobs, eic_values, scan_count, rts_full, options, window)
        });
    }

    #[cfg(all(target_arch = "wasm32", not(target_os = "wasi")))]
    let _ = threads;

    compute_peaks_for_jobs_serial(jobs, eic_values, scan_count, rts_full, options, window)
}

#[cfg(not(all(target_arch = "wasm32", not(target_os = "wasi"))))]
fn compute_peaks_for_jobs_parallel(
    jobs: &[PeakJob],
    eic_values: &[f64],
    scan_count: usize,
    rts_full: &[f64],
    options: &FindPeaksOptions,
    window: FromTo,
) -> Vec<(usize, Peak)> {
    (0..jobs.len())
        .into_par_iter()
        .map(|job_index| {
            compute_peak_for_job_index(
                job_index, jobs, eic_values, scan_count, rts_full, options, window,
            )
        })
        .collect()
}

fn compute_peaks_for_jobs_serial(
    jobs: &[PeakJob],
    eic_values: &[f64],
    scan_count: usize,
    rts_full: &[f64],
    options: &FindPeaksOptions,
    window: FromTo,
) -> Vec<(usize, Peak)> {
    (0..jobs.len())
        .map(|job_index| {
            compute_peak_for_job_index(
                job_index, jobs, eic_values, scan_count, rts_full, options, window,
            )
        })
        .collect()
}

fn compute_peak_for_job_index(
    job_index: usize,
    jobs: &[PeakJob],
    eic_values: &[f64],
    scan_count: usize,
    rts_full: &[f64],
    options: &FindPeaksOptions,
    window: FromTo,
) -> (usize, Peak) {
    let job = &jobs[job_index];
    let row_start = job_index * scan_count;
    let row_end = row_start + scan_count;
    let peak = compute_peak_for_job(
        job,
        &eic_values[row_start..row_end],
        rts_full,
        options,
        window,
    );
    (job.roi_idx, peak)
}
