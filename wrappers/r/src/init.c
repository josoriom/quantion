#include <R.h>
#include <Rinternals.h>
#include <R_ext/Rdynload.h>

SEXP C_bind_rust(SEXP unused);
SEXP C_unbind_rust(SEXP unused);
SEXP C_dispose_sample(SEXP pointer);
SEXP C_parse_mzml(SEXP data);
SEXP C_parse_mzml_path(SEXP path);
SEXP C_parse_ion(SEXP data, SEXP max_cache_size);
SEXP C_parse_ion_path(SEXP path, SEXP max_cache_size);
SEXP C_parse_ion_source(SEXP store_pointer, SEXP max_cache_size);
SEXP C_plan_open(SEXP header);
SEXP C_store_new(void);
SEXP C_store_add(SEXP store_pointer, SEXP offset, SEXP bytes);
SEXP C_write_ion(SEXP file, SEXP out_path, SEXP overwrite, SEXP level, SEXP f32_compress, SEXP block_size,
                 SEXP mz_window, SEXP headers);
SEXP C_write_mzml(SEXP file, SEXP out_path, SEXP overwrite);
SEXP C_get_scans(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit);
SEXP C_plan_scans(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit);
SEXP C_get_scan_metadata(SEXP file, SEXP index, SEXP tags);
SEXP C_plan_scan_metadata(SEXP file, SEXP index);
SEXP C_get_scans_with_arrays(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit, SEXP accessions);
SEXP C_plan_scans_with_arrays(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit, SEXP accessions);
SEXP C_calculate_eic(SEXP file, SEXP mz, SEXP eic);
SEXP C_plan_eic(SEXP file, SEXP mz, SEXP eic);
SEXP C_get_ion_image(SEXP file, SEXP mz, SEXP tolerance, SEXP level);
SEXP C_plan_image(SEXP file, SEXP mz, SEXP tolerance, SEXP level);
SEXP C_find_peaks(SEXP x, SEXP y, SEXP options);
SEXP C_get_peak(SEXP x, SEXP y, SEXP rt, SEXP range, SEXP options);
SEXP C_fit_peak(SEXP x, SEXP y, SEXP rt, SEXP shape, SEXP intensity);
SEXP C_draw_peaks(SEXP heights, SEXP centers, SEXP fwhms, SEXP tails, SEXP from, SEXP to, SEXP points);
SEXP C_find_noise_level(SEXP y);
SEXP C_calculate_baseline(SEXP y, SEXP lambda, SEXP max_iterations);
SEXP C_find_targets(SEXP file, SEXP rts, SEXP mzs, SEXP ranges, SEXP ids, SEXP eic, SEXP options, SEXP threads);
SEXP C_plan_find_targets(SEXP file, SEXP mzs, SEXP eic);
SEXP C_find_transitions(SEXP file, SEXP precursors, SEXP products, SEXP rts, SEXP ranges, SEXP ids, SEXP options, SEXP threads);
SEXP C_find_features(SEXP file, SEXP eic, SEXP grid, SEXP options, SEXP threads);
SEXP C_plan_find_features(SEXP file, SEXP eic, SEXP grid);
SEXP C_deisotope_features(SEXP features, SEXP count, SEXP rt_tolerance, SEXP mz_tolerance);
SEXP C_match_features(SEXP directory, SEXP eic, SEXP grid, SEXP grouping, SEXP options, SEXP threads);
SEXP C_model_load_path(SEXP path);
SEXP C_model_load_bytes(SEXP bytes);
SEXP C_dispose_model(SEXP pointer);
SEXP C_model_metadata(SEXP pointer);
SEXP C_warp_model_load(SEXP pointer);
SEXP C_dispose_warp_model(SEXP pointer);
SEXP C_predict_rts(SEXP warp, SEXP file, SEXP mzs, SEXP rts);
SEXP C_plan_predict_rts(SEXP warp, SEXP file, SEXP mzs, SEXP rts);
SEXP C_calibrate_rts(SEXP warp, SEXP samples, SEXP list_lengths, SEXP ids, SEXP mzs, SEXP rts);

static const R_CallMethodDef CallEntries[] = {
    {"C_bind_rust", (DL_FUNC)&C_bind_rust, 1},
    {"C_unbind_rust", (DL_FUNC)&C_unbind_rust, 1},
    {"C_dispose_sample", (DL_FUNC)&C_dispose_sample, 1},
    {"C_parse_mzml", (DL_FUNC)&C_parse_mzml, 1},
    {"C_parse_mzml_path", (DL_FUNC)&C_parse_mzml_path, 1},
    {"C_parse_ion", (DL_FUNC)&C_parse_ion, 2},
    {"C_parse_ion_path", (DL_FUNC)&C_parse_ion_path, 2},
    {"C_parse_ion_source", (DL_FUNC)&C_parse_ion_source, 2},
    {"C_plan_open", (DL_FUNC)&C_plan_open, 1},
    {"C_store_new", (DL_FUNC)&C_store_new, 0},
    {"C_store_add", (DL_FUNC)&C_store_add, 3},
    {"C_write_ion", (DL_FUNC)&C_write_ion, 8},
    {"C_write_mzml", (DL_FUNC)&C_write_mzml, 3},
    {"C_get_scans", (DL_FUNC)&C_get_scans, 6},
    {"C_plan_scans", (DL_FUNC)&C_plan_scans, 6},
    {"C_get_scan_metadata", (DL_FUNC)&C_get_scan_metadata, 3},
    {"C_plan_scan_metadata", (DL_FUNC)&C_plan_scan_metadata, 2},
    {"C_get_scans_with_arrays", (DL_FUNC)&C_get_scans_with_arrays, 7},
    {"C_plan_scans_with_arrays", (DL_FUNC)&C_plan_scans_with_arrays, 7},
    {"C_calculate_eic", (DL_FUNC)&C_calculate_eic, 3},
    {"C_plan_eic", (DL_FUNC)&C_plan_eic, 3},
    {"C_get_ion_image", (DL_FUNC)&C_get_ion_image, 4},
    {"C_plan_image", (DL_FUNC)&C_plan_image, 4},
    {"C_find_peaks", (DL_FUNC)&C_find_peaks, 3},
    {"C_get_peak", (DL_FUNC)&C_get_peak, 5},
    {"C_fit_peak", (DL_FUNC)&C_fit_peak, 5},
    {"C_draw_peaks", (DL_FUNC)&C_draw_peaks, 7},
    {"C_find_noise_level", (DL_FUNC)&C_find_noise_level, 1},
    {"C_calculate_baseline", (DL_FUNC)&C_calculate_baseline, 3},
    {"C_find_targets", (DL_FUNC)&C_find_targets, 8},
    {"C_plan_find_targets", (DL_FUNC)&C_plan_find_targets, 3},
    {"C_find_transitions", (DL_FUNC)&C_find_transitions, 8},
    {"C_find_features", (DL_FUNC)&C_find_features, 5},
    {"C_plan_find_features", (DL_FUNC)&C_plan_find_features, 3},
    {"C_deisotope_features", (DL_FUNC)&C_deisotope_features, 4},
    {"C_match_features", (DL_FUNC)&C_match_features, 6},
    {"C_model_load_path", (DL_FUNC)&C_model_load_path, 1},
    {"C_model_load_bytes", (DL_FUNC)&C_model_load_bytes, 1},
    {"C_dispose_model", (DL_FUNC)&C_dispose_model, 1},
    {"C_model_metadata", (DL_FUNC)&C_model_metadata, 1},
    {"C_warp_model_load", (DL_FUNC)&C_warp_model_load, 1},
    {"C_dispose_warp_model", (DL_FUNC)&C_dispose_warp_model, 1},
    {"C_predict_rts", (DL_FUNC)&C_predict_rts, 4},
    {"C_plan_predict_rts", (DL_FUNC)&C_plan_predict_rts, 4},
    {"C_calibrate_rts", (DL_FUNC)&C_calibrate_rts, 6},
    {NULL, NULL, 0}};

void R_init_quantion(DllInfo *dll)
{
  R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
  R_useDynamicSymbols(dll, FALSE);
}
