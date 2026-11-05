#pragma once

/* Generated with cbindgen:0.29.4 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define QUANTION_ABI_VERSION 2

#define QUANTION_BRIDGE_MAGIC 1112821329

#define QUANTION_BRIDGE_LAYOUT_VERSION 1

#define QUANTION_BRIDGE_HEADER_BYTES 32

#define QUANTION_SECTION_ENTRY_BYTES 24

#define QUANTION_PAYLOAD_SCANS 1

#define QUANTION_PAYLOAD_ION_IMAGE 2

#define QUANTION_PAYLOAD_PEAKS 3

#define QUANTION_PAYLOAD_FEATURES 4

#define QUANTION_PAYLOAD_FIT_RESULT 6

#define QUANTION_PAYLOAD_CONSENSUS_FEATURES 8

#define QUANTION_PAYLOAD_FOUND_FEATURES 9

#define QUANTION_PAYLOAD_WARP_TARGETS 10

#define QUANTION_PAYLOAD_MODEL_METADATA 11

#define QUANTION_PAYLOAD_SCAN_METADATA 12

#define QUANTION_ELEMENT_F64 1

#define QUANTION_ELEMENT_U32 2

#define QUANTION_ELEMENT_U64 3

#define QUANTION_ELEMENT_U8 4

#define QUANTION_SECTION_POINT_STARTS 1

#define QUANTION_SECTION_MZ 2

#define QUANTION_SECTION_INTENSITY 3

#define QUANTION_SECTION_RT 4

#define QUANTION_SECTION_RT_SECONDS 5

#define QUANTION_SECTION_BASE_PEAK_MZ 6

#define QUANTION_SECTION_SELECTED_ION_MZ 7

#define QUANTION_SECTION_BASE_PEAK_INT 8

#define QUANTION_SECTION_TOTAL_ION_CURRENT 9

#define QUANTION_SECTION_MS_LEVEL 10

#define QUANTION_SECTION_POLARITY 11

#define QUANTION_SECTION_POSITION_X 12

#define QUANTION_SECTION_POSITION_Y 13

#define QUANTION_SECTION_POSITION_Z 14

#define QUANTION_SECTION_IMAGE_SHAPE 15

#define QUANTION_SECTION_IMAGE_DATA 16

#define QUANTION_SECTION_IMAGE_COUNTS 17

#define QUANTION_SECTION_PEAK_FROM 18

#define QUANTION_SECTION_PEAK_TO 19

#define QUANTION_SECTION_PEAK_RT 20

#define QUANTION_SECTION_PEAK_INTEGRAL 21

#define QUANTION_SECTION_PEAK_INTENSITY 22

#define QUANTION_SECTION_PEAK_POINT_COUNT 23

#define QUANTION_SECTION_FEATURE_MZ 26

#define QUANTION_SECTION_FEATURE_RT 27

#define QUANTION_SECTION_FEATURE_FROM 28

#define QUANTION_SECTION_FEATURE_TO 29

#define QUANTION_SECTION_FEATURE_INTENSITY 30

#define QUANTION_SECTION_FEATURE_INTEGRAL 31

#define QUANTION_SECTION_FEATURE_POINT_COUNT 32

#define QUANTION_SECTION_FIT_SHAPE 46

#define QUANTION_SECTION_FIT_HEIGHT 47

#define QUANTION_SECTION_FIT_CENTER 48

#define QUANTION_SECTION_FIT_FWHM 49

#define QUANTION_SECTION_FIT_TAIL 50

#define QUANTION_SECTION_FIT_R2 51

#define QUANTION_SECTION_CONSENSUS_MZ 62

#define QUANTION_SECTION_CONSENSUS_RT 63

#define QUANTION_SECTION_CONSENSUS_FROM 64

#define QUANTION_SECTION_CONSENSUS_TO 65

#define QUANTION_SECTION_CONSENSUS_INTENSITY 66

#define QUANTION_SECTION_CONSENSUS_INTEGRAL 67

#define QUANTION_SECTION_CONSENSUS_FREQUENCY 68

#define QUANTION_SECTION_FOUND_ID_STARTS 69

#define QUANTION_SECTION_FOUND_ID_BYTES 70

#define QUANTION_SECTION_FOUND_MZ 71

#define QUANTION_SECTION_FOUND_RT 72

#define QUANTION_SECTION_FOUND_FROM 73

#define QUANTION_SECTION_FOUND_TO 74

#define QUANTION_SECTION_FOUND_INTENSITY 75

#define QUANTION_SECTION_FOUND_INTEGRAL 76

#define QUANTION_SECTION_FOUND_POINT_COUNT 77

#define QUANTION_SECTION_FOUND_NOISE 78

#define QUANTION_SECTION_WARP_TARGET_ID_STARTS 79

#define QUANTION_SECTION_WARP_TARGET_ID_BYTES 80

#define QUANTION_SECTION_WARP_TARGET_MZ 81

#define QUANTION_SECTION_WARP_TARGET_RT 82

#define QUANTION_SECTION_MODEL_METADATA_KEY_STARTS 83

#define QUANTION_SECTION_MODEL_METADATA_KEY_BYTES 84

#define QUANTION_SECTION_MODEL_METADATA_VALUE_STARTS 85

#define QUANTION_SECTION_MODEL_METADATA_VALUE_BYTES 86

#define QUANTION_SECTION_CONSENSUS_POINT_COUNT 87

#define QUANTION_SECTION_SCAN_ARRAY_STARTS 88

#define QUANTION_SECTION_SCAN_ARRAY_VALUES 89

#define QUANTION_SECTION_SCAN_INDEX 90

#define QUANTION_SECTION_SCAN_METADATA_PATH_STARTS 91

#define QUANTION_SECTION_SCAN_METADATA_PATH_BYTES 92

#define QUANTION_SECTION_SCAN_METADATA_KIND_STARTS 93

#define QUANTION_SECTION_SCAN_METADATA_KIND_BYTES 94

#define QUANTION_SECTION_SCAN_METADATA_ACCESSION_STARTS 95

#define QUANTION_SECTION_SCAN_METADATA_ACCESSION_BYTES 96

#define QUANTION_SECTION_SCAN_METADATA_ACCESSION_MISSING 97

#define QUANTION_SECTION_SCAN_METADATA_NAME_STARTS 98

#define QUANTION_SECTION_SCAN_METADATA_NAME_BYTES 99

#define QUANTION_SECTION_SCAN_METADATA_VALUE_STARTS 100

#define QUANTION_SECTION_SCAN_METADATA_VALUE_BYTES 101

#define QUANTION_SECTION_SCAN_METADATA_VALUE_MISSING 102

#define QUANTION_SECTION_SCAN_METADATA_UNIT_STARTS 103

#define QUANTION_SECTION_SCAN_METADATA_UNIT_BYTES 104

#define QUANTION_SECTION_SCAN_METADATA_UNIT_MISSING 105

#define QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_STARTS 106

#define QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_BYTES 107

#define QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_MISSING 108

typedef struct ImageSession ImageSession;

typedef struct ModelHandle ModelHandle;

typedef struct ParsedFile ParsedFile;

typedef struct WarpModelHandle WarpModelHandle;

typedef struct Buf {
  uint8_t *ptr;
  uintptr_t len;
} Buf;

typedef struct CPeakOptions {
  double min_integral;
  double min_intensity;
  int min_peak_width_points;
  int shape;
  double noise;
  int auto_noise;
  int auto_baseline;
  int max_iterations;
  int allow_overlap;
  double lambda;
  double min_snr;
  double min_r2;
  int kernel_size;
} CPeakOptions;

uint32_t quantion_abi_version(void);

uintptr_t quantion_sizeof_peak_options(void);

#if !(defined(QUANTION_TARGET_WASM32) && !defined(QUANTION_TARGET_WASI))
/**
 * Open an ion file whose bytes the caller supplies on demand.
 *
 * The library never reads a file or a socket itself: every time it needs a slice
 * it calls `read`. Use this for a file that lives somewhere the library cannot
 * reach, such as a URL.
 *
 * # Safety
 * `read` must stay callable, and `context` must stay valid, until the returned
 * handle is freed with `free_mzml`.
 */
int parse_ion_source(int32_t (*read)(void *context, uint64_t offset, uint64_t length, uint8_t *dest),
                     void *context,
                     uintptr_t max_cache_size,
                     struct ParsedFile **out);
#endif

#if (defined(QUANTION_TARGET_WASM32) && !defined(QUANTION_TARGET_WASI))
extern int32_t range_read(uint32_t source_id,
                          uint32_t offset_lo,
                          uint32_t offset_hi,
                          uint32_t len,
                          uint8_t *dest_ptr);
#endif

uint8_t *alloc(uintptr_t size);

/**
 * Free memory returned by `alloc`.
 *
 * # Safety
 * `ptr_raw` must be a live pointer from `alloc(size)`.
 * `size` must match the `alloc` call.
 * After this call, `ptr_raw` is invalid.
 */
void free_(uint8_t *ptr_raw, uintptr_t size);

/**
 * Free a `ParsedFile` handle.
 *
 * # Safety
 * `handle` must be a valid pointer returned by this library.
 * Do not free the same handle twice.
 * After this call, `handle` is invalid.
 */
void free_mzml(struct ParsedFile *handle);

/**
 * Parse mzML bytes and store the result in `dest`.
 *
 * # Safety
 * `data_ptr` must point to `data_len` readable bytes.
 * `dest` must be a valid writable pointer to `*mut ParsedFile`.
 * On success, `*dest` must be freed with `free_mzml`.
 */
int parse_mzml(const uint8_t *data_ptr, uintptr_t data_len, struct ParsedFile **dest);

#if (defined(QUANTION_TARGET_WASM32) && !defined(QUANTION_TARGET_WASI))
int parse_ion_url(uint32_t source_id, uintptr_t cache_bytes, struct ParsedFile **dest);
#endif

/**
 * Parse binary data and store the result in `dest`.
 *
 * # Safety
 * `data_ptr` must point to `data_len` readable bytes.
 * `dest` must be a valid writable pointer to `*mut ParsedFile`.
 * On success, `*dest` must be freed with `free_mzml`.
 */
int parse_bin(const uint8_t *data_ptr,
              uintptr_t data_len,
              uintptr_t max_cache_size,
              struct ParsedFile **dest);

#if !(defined(QUANTION_TARGET_WASM32) && !defined(QUANTION_TARGET_WASI))
int parse_ion_path(const char *path_ptr, uintptr_t max_cache_size, struct ParsedFile **dest);
#endif

/**
 * Plan the byte ranges an open needs.
 *
 * # Safety
 * `header_ptr` must point to at least `header_len` readable bytes, and `header_len` must be at
 * least 1024 so the header can be parsed.
 */
int plan_open(const uint8_t *header_ptr, uintptr_t header_len, struct Buf *out);

/**
 * Write the noise window width and the noise intensity for an intensity array.
 *
 * # Safety
 * `y_ptr` must point to `len` readable `f32` values.
 * `out_width` and `out_intensity` must be valid writable pointers.
 */
int find_noise_level(const float *y_ptr,
                     uintptr_t len,
                     uintptr_t *out_width,
                     double *out_intensity);

int plan_image(struct ParsedFile *h,
               double target,
               double tolerance,
               uint8_t level,
               struct Buf *out);

int image_begin(struct ParsedFile *h,
                double target,
                double tolerance,
                uint8_t level,
                uintptr_t *out_session);

int image_scan_count(struct ImageSession *session, uintptr_t *out_count);

int image_ranges(struct ParsedFile *h,
                 struct ImageSession *session,
                 uintptr_t from,
                 uintptr_t count,
                 struct Buf *out);

int image_fold(struct ParsedFile *h, struct ImageSession *session, uintptr_t from, uintptr_t count);

int image_finish(struct ImageSession *session, struct Buf *out);

void image_free(struct ImageSession *session);

int model_load_path(const uint8_t *path_ptr,
                    uintptr_t path_len,
                    struct ModelHandle **out,
                    struct Buf *message);

int model_load_bytes(const uint8_t *model_ptr,
                     uintptr_t model_len,
                     struct ModelHandle **out,
                     struct Buf *message);

void model_free(struct ModelHandle *model);

int model_metadata(const struct ModelHandle *model, struct Buf *out);

/**
 * Write the error of the last failed call on this thread to `out` as UTF-8 text:
 * code, kind, function, argument and message, separated by the unit separator (0x1F).
 *
 * # Safety
 * `out` must be a valid writable `Buf` pointer.
 */
int quantion_last_error(struct Buf *out);

/**
 * Open an mzML file from a path.
 *
 * # Safety
 * `path` must be a valid NUL-terminated string. `dest` must be a valid writable pointer.
 */
int parse_mzml_path(const char *path, struct ParsedFile **dest);

/**
 * Write a sample as .ion. A null `out_path` or a URL returns the bytes in `out`; a path writes a file.
 * For a URL the headers are checked here and the caller uploads the bytes with its own HTTP client. `headers` holds name, value, name, value… each ending in a NUL byte.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out_path` must be null or NUL-terminated.
 * `headers` must point to `headers_len` readable bytes. `out` must be a valid writable `Buf`.
 */
int write_ion(struct ParsedFile *handle,
              const char *out_path,
              uint8_t overwrite,
              uint8_t level,
              uint8_t f32_compress,
              uint32_t block_size,
              uint32_t mz_window,
              const uint8_t *headers,
              uintptr_t headers_len,
              struct Buf *out);

/**
 * Write a sample as mzML. A null `out_path` returns the bytes in `out`.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out_path` must be null or NUL-terminated.
 * `out` must be a valid writable `Buf`.
 */
int write_mzml(struct ParsedFile *handle, const char *out_path, uint8_t overwrite, struct Buf *out);

/**
 * Find every peak in a trace.
 *
 * # Safety
 * `x` and `y` must point to `count` readable values. `options` must be null or valid.
 * `out` must be a valid writable `Buf`.
 */
int find_peaks(const double *x,
               const double *y,
               uintptr_t count,
               const struct CPeakOptions *options,
               struct Buf *out);

/**
 * Find the one peak near `rt`. A `range` of 0 searches the whole trace.
 *
 * # Safety
 * `x` and `y` must point to `count` readable values. `options` must be null or valid.
 * `out` must be a valid writable `Buf`.
 */
int get_peak(const double *x,
             const double *y,
             uintptr_t count,
             double rt,
             double range,
             const struct CPeakOptions *options,
             struct Buf *out);

/**
 * Fit a curve to the peak at `rt`. `shape` is 0 for Gaussian, 1 for EMG.
 * An `intensity` of 0 takes it from the peak found. No fit gives an empty record.
 *
 * # Safety
 * `x` and `y` must point to `count` readable values. `out` must be a valid writable `Buf`.
 */
int fit_peak(const double *x,
             const double *y,
             uintptr_t count,
             double rt,
             int shape,
             double intensity,
             struct Buf *out);

/**
 * Draw peak curves into one chromatogram. `from` and `to` both 0 fit the range around the peaks.
 *
 * # Safety
 * `heights`, `centers`, `fwhms` and `tails` must point to `count` readable values.
 * `out_x` and `out_y` must be valid writable `Buf` pointers.
 */
int draw_peaks(const double *heights,
               const double *centers,
               const double *fwhms,
               const double *tails,
               uintptr_t count,
               double from,
               double to,
               uintptr_t points,
               struct Buf *out_x,
               struct Buf *out_y);

/**
 * Estimate a baseline with airPLS.
 *
 * # Safety
 * `y` must point to `count` readable values. `out` must be a valid writable `Buf`.
 */
int calculate_baseline(const double *y,
                       uintptr_t count,
                       double lambda,
                       uintptr_t max_iterations,
                       struct Buf *out);

/**
 * Extract the EIC of one m/z. `time_unit` is 0 for minutes, 1 for seconds.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out_x` and `out_y` must be valid writable `Buf`s.
 */
int calculate_eic(struct ParsedFile *handle,
                  double mz,
                  double from,
                  double to,
                  double ppm_tolerance,
                  double mz_tolerance,
                  uint8_t time_unit,
                  struct Buf *out_x,
                  struct Buf *out_y);

/**
 * Plan the byte ranges `calculate_eic` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int plan_eic(struct ParsedFile *handle,
             double mz,
             double from,
             double to,
             double ppm_tolerance,
             double mz_tolerance,
             uint8_t time_unit,
             struct Buf *out);

/**
 * Read scans. `query_type`: 0 rt range, 1 closest rt, 2 m/z range, 3 closest m/z.
 * `time_unit` is 0 for minutes, 1 for seconds.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int get_scans(struct ParsedFile *handle,
              uint8_t query_type,
              double from,
              double to,
              uint8_t level,
              uint8_t time_unit,
              struct Buf *out);

/**
 * Read chosen arrays of each scan. `accessions` holds the number of each array, such as 1000786.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `accessions` must hold `array_count` numbers. `out` must be a valid writable `Buf`.
 */
int get_scans_with_arrays(struct ParsedFile *handle,
                          uint8_t query_type,
                          double from,
                          double to,
                          uint8_t level,
                          uint8_t time_unit,
                          const uint32_t *accessions,
                          uintptr_t array_count,
                          struct Buf *out);

/**
 * Plan the byte ranges `get_scans_with_arrays` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `accessions` must hold `array_count` numbers. `out` must be a valid writable `Buf`.
 */
int plan_scans_with_arrays(struct ParsedFile *handle,
                           uint8_t query_type,
                           double from,
                           double to,
                           uint8_t level,
                           uint8_t time_unit,
                           const uint32_t *accessions,
                           uintptr_t array_count,
                           struct Buf *out);

/**
 * Plan the byte ranges `get_scans` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int plan_scans(struct ParsedFile *handle,
               uint8_t query_type,
               double from,
               double to,
               uint8_t level,
               uint8_t time_unit,
               struct Buf *out);

/**
 * Read the attributes, cvParams and userParams of one scan, one row each. `tags` holds tag names split by the unit separator 0x1F; null or empty keeps every tag.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `tags` must be null or NUL-terminated text. `out` must be a valid writable `Buf`.
 */
int get_scan_metadata(struct ParsedFile *handle,
                      uintptr_t index,
                      const char *tags,
                      struct Buf *out);

/**
 * Plan the byte ranges `get_scan_metadata` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int plan_scan_metadata(struct ParsedFile *handle, uintptr_t index, struct Buf *out);

/**
 * Build the ion image of one m/z. Only .ion files.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int get_ion_image(struct ParsedFile *handle,
                  double mz,
                  double tolerance,
                  uint8_t level,
                  struct Buf *out);

/**
 * Find the peak of each target in its EIC. A NaN range means no range: the whole EIC is searched.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `rts`, `mzs` and `ranges` must point to `count` values.
 * Ids follow the `read_ids` layout. `options` must be null or valid. `out` must be a valid writable `Buf`.
 */
int find_targets(struct ParsedFile *handle,
                 const double *rts,
                 const double *mzs,
                 const double *ranges,
                 const uint32_t *ids_offsets,
                 const uint32_t *ids_lengths,
                 const uint8_t *ids_text,
                 uintptr_t ids_text_len,
                 uintptr_t count,
                 double from,
                 double to,
                 double ppm_tolerance,
                 double mz_tolerance,
                 uint8_t time_unit,
                 const struct CPeakOptions *options,
                 uintptr_t threads,
                 struct Buf *out);

/**
 * Plan the byte ranges `find_targets` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `mzs` must point to `count` values.
 * `out` must be a valid writable `Buf`.
 */
int plan_find_targets(struct ParsedFile *handle,
                      const double *mzs,
                      uintptr_t count,
                      double from,
                      double to,
                      double ppm_tolerance,
                      double mz_tolerance,
                      uint8_t time_unit,
                      struct Buf *out);

/**
 * Find the peak of each SRM transition. Needs the whole file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. The number arrays must point to `count` values.
 * Ids follow the `read_ids` layout. `options` must be null or valid. `out` must be a valid writable `Buf`.
 */
int find_transitions(struct ParsedFile *handle,
                     const double *precursors,
                     const double *products,
                     const double *rts,
                     const double *ranges,
                     const uint32_t *ids_offsets,
                     const uint32_t *ids_lengths,
                     const uint8_t *ids_text,
                     uintptr_t ids_text_len,
                     uintptr_t count,
                     const struct CPeakOptions *options,
                     uintptr_t threads,
                     struct Buf *out);

/**
 * Find every feature in one file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `options` must be null or valid. `out` must be a valid writable `Buf`.
 */
int find_features(struct ParsedFile *handle,
                  double from,
                  double to,
                  double ppm_tolerance,
                  double mz_tolerance,
                  uint8_t time_unit,
                  double grid_start,
                  double grid_end,
                  double grid_step_size,
                  const struct CPeakOptions *options,
                  uintptr_t threads,
                  struct Buf *out);

/**
 * Plan the byte ranges `find_features` reads from an .ion file.
 *
 * # Safety
 * `handle` must be a valid `ParsedFile`. `out` must be a valid writable `Buf`.
 */
int plan_find_features(struct ParsedFile *handle,
                       double from,
                       double to,
                       double ppm_tolerance,
                       double mz_tolerance,
                       uint8_t time_unit,
                       double grid_start,
                       double grid_end,
                       double grid_step_size,
                       struct Buf *out);

/**
 * Remove 13C isotope peaks from a feature list. `features` holds 8 values per feature, in this order:
 * mz, rt, from, to, intensity, integral, n_points, frequency (NaN when there is none).
 *
 * # Safety
 * `features` must point to `count * 8` readable values. `out` must be a valid writable `Buf`.
 */
int deisotope_features(const double *features,
                       uintptr_t count,
                       double rt_tolerance,
                       double mz_tolerance,
                       struct Buf *out);

#if !(defined(QUANTION_TARGET_WASM32) && !defined(QUANTION_TARGET_WASI))
/**
 * Find features in every file of a folder and match them.
 *
 * # Safety
 * `directory` must be a valid NUL-terminated string. `options` must be null or valid.
 * `out` must be a valid writable `Buf`.
 */
int match_features(const char *directory,
                   double from,
                   double to,
                   double ppm_tolerance,
                   double mz_tolerance,
                   uint8_t time_unit,
                   double grid_start,
                   double grid_end,
                   double grid_step_size,
                   double grouping_ppm_tolerance,
                   double grouping_mz_tolerance,
                   double grouping_rt_tolerance,
                   double grouping_min_frequency,
                   const struct CPeakOptions *options,
                   uintptr_t threads,
                   struct Buf *out);
#endif

int warp_model_load(const struct ModelHandle *model,
                    struct WarpModelHandle **out,
                    struct Buf *message);

void warp_model_free(struct WarpModelHandle *warp);

int predict_rts_from_file(const struct WarpModelHandle *warp,
                          const struct ParsedFile *h,
                          const double *mzs_ptr,
                          const double *rts_ptr,
                          uintptr_t n,
                          struct Buf *out);

int plan_predict_rts(const struct WarpModelHandle *warp,
                     struct ParsedFile *h,
                     const double *mzs_ptr,
                     const double *rts_ptr,
                     uintptr_t n,
                     struct Buf *out);

int calibrate_rts(const struct WarpModelHandle *warp,
                  const struct ParsedFile *const *files,
                  uintptr_t file_count,
                  const uintptr_t *list_lengths,
                  uintptr_t list_count,
                  const uint32_t *ids_off,
                  const uint32_t *ids_len,
                  const uint8_t *ids_buf,
                  uintptr_t ids_buf_len,
                  const double *mzs_ptr,
                  const double *rts_ptr,
                  uintptr_t target_count,
                  struct Buf *out);
