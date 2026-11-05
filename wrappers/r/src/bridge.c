#include <R.h>
#include <Rinternals.h>
#include <R_ext/Rdynload.h>
#include <stdint.h>
#include <stddef.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>
#include <limits.h>
#include "include/quantion.h"

static int library_is_bound = 0;

static void stop_with_last_error(int code)
{
  Buf text = (Buf){0};
  quantion_last_error(&text);
  SEXP message = PROTECT(Rf_allocVector(STRSXP, 1));
  if (text.len > 0 && text.len <= (size_t)INT_MAX)
    SET_STRING_ELT(message, 0, Rf_mkCharLenCE((const char *)text.ptr, (int)text.len, CE_UTF8));
  else
    SET_STRING_ELT(message, 0, Rf_mkChar(""));
  if (text.ptr)
    free_(text.ptr, text.len);
  SEXP package = PROTECT(R_FindNamespace(Rf_mkString("quantion")));
  SEXP call = PROTECT(Rf_lang3(Rf_install(".stop_with_rust_error"), message, Rf_ScalarInteger(code)));
  Rf_eval(call, package);
  UNPROTECT(3);
}

static void check_code(int code)
{
  if (code != 0)
    stop_with_last_error(code);
}

static void check_bound(void)
{
  if (!library_is_bound)
    error("quantion: the native library is not loaded; did .onLoad() run?");
}

SEXP C_bind_rust(SEXP unused)
{
  (void)unused;
  if (quantion_abi_version() != QUANTION_ABI_VERSION)
    error("quantion: ABI version mismatch: recompile the R wrapper");
  if (quantion_sizeof_peak_options() != sizeof(CPeakOptions))
    error("quantion: CPeakOptions size mismatch: native library and R wrapper are out of sync");
  library_is_bound = 1;
  return R_NilValue;
}

SEXP C_unbind_rust(SEXP unused)
{
  (void)unused;
  library_is_bound = 0;
  return R_NilValue;
}

typedef struct
{
  uint32_t element_type;
  uint64_t byte_offset;
  uint64_t byte_length;
} bridge_section;

typedef struct
{
  const unsigned char *raw;
  uint64_t total_bytes;
  uint16_t payload_kind;
  uint64_t record_count;
  uint32_t section_count;
  bridge_section sections[64];
  uint32_t section_ids[64];
} bridge_view;

static uint16_t read_u16(const unsigned char *raw, uint64_t at)
{
  uint16_t value;
  memcpy(&value, raw + at, sizeof(value));
  return value;
}

static uint32_t read_u32(const unsigned char *raw, uint64_t at)
{
  uint32_t value;
  memcpy(&value, raw + at, sizeof(value));
  return value;
}

static uint64_t read_u64(const unsigned char *raw, uint64_t at)
{
  uint64_t value;
  memcpy(&value, raw + at, sizeof(value));
  return value;
}

static size_t size_of_element(uint32_t element_type)
{
  if (element_type == QUANTION_ELEMENT_F64)
    return 8;
  if (element_type == QUANTION_ELEMENT_U32)
    return 4;
  if (element_type == QUANTION_ELEMENT_U64)
    return 8;
  if (element_type == QUANTION_ELEMENT_U8)
    return 1;
  return 0;
}

static uint32_t expected_element_type(uint32_t section_id)
{
  if (section_id == QUANTION_SECTION_POINT_STARTS)
    return QUANTION_ELEMENT_U64;
  if (section_id == QUANTION_SECTION_IMAGE_SHAPE || section_id == QUANTION_SECTION_IMAGE_COUNTS)
    return QUANTION_ELEMENT_U32;
  if (section_id <= QUANTION_SECTION_IMAGE_DATA)
    return QUANTION_ELEMENT_F64;
  return 0;
}

static void read_bridge_view(bridge_view *bridge, const unsigned char *raw, size_t len)
{
  if (len < QUANTION_BRIDGE_HEADER_BYTES)
    error("quantion bridge: buffer is shorter than the header");

  uint64_t total = (uint64_t)len;
  if (read_u32(raw, 0) != QUANTION_BRIDGE_MAGIC)
    error("quantion bridge: magic does not match");
  if (read_u16(raw, 4) != QUANTION_BRIDGE_LAYOUT_VERSION)
    error("quantion bridge: layout version is not supported");

  uint32_t count = read_u32(raw, 8);
  if (read_u32(raw, 12) != QUANTION_BRIDGE_HEADER_BYTES)
    error("quantion bridge: section table does not start after the header");
  if (read_u64(raw, 16) != total)
    error("quantion bridge: total bytes does not match the buffer");
  if (count > 64)
    error("quantion bridge: too many sections");
  if ((uint64_t)count * QUANTION_SECTION_ENTRY_BYTES > total - QUANTION_BRIDGE_HEADER_BYTES)
    error("quantion bridge: section table does not fit");

  bridge->raw = raw;
  bridge->total_bytes = total;
  bridge->payload_kind = read_u16(raw, 6);
  bridge->record_count = read_u64(raw, 24);
  bridge->section_count = count;

  uint64_t reach = QUANTION_BRIDGE_HEADER_BYTES + (uint64_t)count * QUANTION_SECTION_ENTRY_BYTES;
  for (uint32_t index = 0; index < count; index++)
  {
    uint64_t start = QUANTION_BRIDGE_HEADER_BYTES + (uint64_t)index * QUANTION_SECTION_ENTRY_BYTES;
    uint32_t section_id = read_u32(raw, start);
    uint32_t element_type = read_u32(raw, start + 4);
    uint64_t offset = read_u64(raw, start + 8);
    uint64_t length = read_u64(raw, start + 16);

    for (uint32_t seen = 0; seen < index; seen++)
      if (bridge->section_ids[seen] == section_id)
        error("quantion bridge: section %u appears twice", section_id);
    if (offset > total || length > total - offset)
      error("quantion bridge: section %u runs past the end", section_id);
    if (offset % 8 != 0)
      error("quantion bridge: section %u is not aligned to eight bytes", section_id);
    if (offset < reach)
      error("quantion bridge: section %u overlaps the section before it", section_id);

    size_t element_size = size_of_element(element_type);
    if (element_size == 0)
      error("quantion bridge: section %u has an unknown element type", section_id);
    if (length % (uint64_t)element_size != 0)
      error("quantion bridge: section %u is not a whole number of elements", section_id);

    uint32_t expected = expected_element_type(section_id);
    if (expected != 0 && expected != element_type)
      error("quantion bridge: section %u has the wrong element type", section_id);

    reach = offset + length;
    bridge->section_ids[index] = section_id;
    bridge->sections[index].element_type = element_type;
    bridge->sections[index].byte_offset = offset;
    bridge->sections[index].byte_length = length;
  }
}

static const bridge_section *find_section(const bridge_view *bridge, uint32_t section_id)
{
  for (uint32_t index = 0; index < bridge->section_count; index++)
    if (bridge->section_ids[index] == section_id)
      return &bridge->sections[index];
  error("quantion bridge: section %u is missing", section_id);
  return NULL;
}

static SEXP number_column(const bridge_view *bridge, uint32_t section_id, R_xlen_t count)
{
  const bridge_section *section = find_section(bridge, section_id);
  if (section->byte_length / 8 != (uint64_t)count)
    error("quantion bridge: section %u does not match the record count", section_id);
  SEXP column = PROTECT(Rf_allocVector(REALSXP, count));
  if (count > 0)
    memcpy(REAL(column), bridge->raw + section->byte_offset, (size_t)count * 8);
  UNPROTECT(1);
  return column;
}

static SEXP point_column(const bridge_view *bridge, uint32_t section_id,
                         const unsigned char *starts_at, R_xlen_t count)
{
  const bridge_section *values = find_section(bridge, section_id);
  SEXP column = PROTECT(Rf_allocVector(VECSXP, count));
  for (R_xlen_t index = 0; index < count; index++)
  {
    uint64_t from = read_u64(starts_at, (uint64_t)index * 8);
    uint64_t to = read_u64(starts_at, (uint64_t)(index + 1) * 8);
    if (to < from || to > values->byte_length / 8)
      error("quantion bridge: point starts run past section %u", section_id);
    R_xlen_t points = (R_xlen_t)(to - from);
    SEXP scan = PROTECT(Rf_allocVector(REALSXP, points));
    if (points > 0)
      memcpy(REAL(scan), bridge->raw + values->byte_offset + from * 8, (size_t)points * 8);
    SET_VECTOR_ELT(column, index, scan);
    UNPROTECT(1);
  }
  UNPROTECT(1);
  return column;
}

static SEXP set_data_frame(SEXP columns, SEXP names, R_xlen_t rows)
{
  Rf_setAttrib(columns, R_NamesSymbol, names);
  SEXP row_names = PROTECT(Rf_allocVector(INTSXP, 2));
  INTEGER(row_names)[0] = NA_INTEGER;
  INTEGER(row_names)[1] = -(int)rows;
  Rf_setAttrib(columns, R_RowNamesSymbol, row_names);
  UNPROTECT(1);
  Rf_setAttrib(columns, R_ClassSymbol, Rf_mkString("data.frame"));
  return columns;
}

struct scans_bridge_data
{
  const unsigned char *ptr;
  size_t len;
};

static void scans_bridge_free(void *data, Rboolean jump)
{
  (void)jump;
  struct scans_bridge_data *held = (struct scans_bridge_data *)data;
  if (held->ptr)
    free_((unsigned char *)held->ptr, held->len);
  held->ptr = NULL;
}


struct buf_copy_data
{
  const unsigned char *first_ptr;
  size_t first_len;
  const unsigned char *second_ptr;
  size_t second_len;
  int as_raw;
};

static SEXP copy_one_buf(const unsigned char *ptr, size_t len, int as_raw)
{
  if (as_raw)
  {
    SEXP out = Rf_allocVector(RAWSXP, (R_xlen_t)len);
    if (len > 0)
      memcpy(RAW(out), ptr, len);
    return out;
  }
  R_xlen_t count = (R_xlen_t)(len / 8);
  SEXP out = Rf_allocVector(REALSXP, count);
  if (count > 0)
    memcpy(REAL(out), ptr, (size_t)count * 8);
  return out;
}

static SEXP buf_copy_build(void *data)
{
  struct buf_copy_data *held = (struct buf_copy_data *)data;
  if (held->second_ptr == NULL)
    return copy_one_buf(held->first_ptr, held->first_len, held->as_raw);

  SEXP x = PROTECT(copy_one_buf(held->first_ptr, held->first_len, held->as_raw));
  SEXP y = PROTECT(copy_one_buf(held->second_ptr, held->second_len, held->as_raw));
  SEXP pair = PROTECT(Rf_allocVector(VECSXP, 2));
  SET_VECTOR_ELT(pair, 0, x);
  SET_VECTOR_ELT(pair, 1, y);
  SEXP names = PROTECT(Rf_allocVector(STRSXP, 2));
  SET_STRING_ELT(names, 0, Rf_mkChar("x"));
  SET_STRING_ELT(names, 1, Rf_mkChar("y"));
  Rf_setAttrib(pair, R_NamesSymbol, names);
  UNPROTECT(4);
  return pair;
}

static void buf_copy_free(void *data, Rboolean jump)
{
  (void)jump;
  struct buf_copy_data *held = (struct buf_copy_data *)data;
  if (held->first_ptr)
    free_((unsigned char *)held->first_ptr, held->first_len);
  if (held->second_ptr)
    free_((unsigned char *)held->second_ptr, held->second_len);
  held->first_ptr = NULL;
  held->second_ptr = NULL;
}

static SEXP ranges_build(void *data)
{
  struct buf_copy_data *held = (struct buf_copy_data *)data;
  size_t count = held->first_len / 16;
  SEXP result = PROTECT(allocMatrix(REALSXP, (int)count, 2));
  double *values = REAL(result);
  for (size_t index = 0; index < count; index++)
  {
    uint64_t offset = 0;
    uint64_t length = 0;
    memcpy(&offset, held->first_ptr + index * 16, 8);
    memcpy(&length, held->first_ptr + index * 16 + 8, 8);
    values[index] = (double)offset;
    values[count + index] = (double)length;
  }
  UNPROTECT(1);
  return result;
}


typedef struct
{
  const char *name;
  uint32_t id;
  uint32_t second_id;
  int kind;
  uint32_t missing_id;
} record_column;

#define COLUMN_NUMBER 0
#define COLUMN_COUNT 1
#define COLUMN_TEXT 2
#define COLUMN_OPTIONAL_TEXT 3

static const record_column PEAK_COLUMNS[] = {
    {"from", 18, 0, COLUMN_NUMBER}, {"to", 19, 0, COLUMN_NUMBER},
    {"rt", 20, 0, COLUMN_NUMBER}, {"integral", 21, 0, COLUMN_NUMBER},
    {"intensity", 22, 0, COLUMN_NUMBER}, {"n_points", 23, 0, COLUMN_COUNT}};

static const record_column FEATURE_COLUMNS[] = {
    {"mz", 26, 0, COLUMN_NUMBER}, {"rt", 27, 0, COLUMN_NUMBER},
    {"from", 28, 0, COLUMN_NUMBER}, {"to", 29, 0, COLUMN_NUMBER},
    {"intensity", 30, 0, COLUMN_NUMBER}, {"integral", 31, 0, COLUMN_NUMBER},
    {"n_points", 32, 0, COLUMN_COUNT}};

static const record_column FIT_COLUMNS[] = {
    {"height", 47, 0, COLUMN_NUMBER},
    {"center", 48, 0, COLUMN_NUMBER}, {"fwhm", 49, 0, COLUMN_NUMBER},
    {"tail", 50, 0, COLUMN_NUMBER}, {"r2", 51, 0, COLUMN_NUMBER}};

static const record_column CONSENSUS_COLUMNS[] = {
    {"mz", 62, 0, COLUMN_NUMBER}, {"rt", 63, 0, COLUMN_NUMBER},
    {"from", 64, 0, COLUMN_NUMBER}, {"to", 65, 0, COLUMN_NUMBER},
    {"intensity", 66, 0, COLUMN_NUMBER}, {"integral", 67, 0, COLUMN_NUMBER},
    {"n_points", 87, 0, COLUMN_COUNT}, {"frequency", 68, 0, COLUMN_NUMBER}};

static const record_column WARP_TARGET_COLUMNS[] = {
    {"id", 79, 80, COLUMN_TEXT}, {"mz", 81, 0, COLUMN_NUMBER},
    {"rt", 82, 0, COLUMN_NUMBER}};

static const record_column MODEL_METADATA_COLUMNS[] = {
    {"key", 83, 84, COLUMN_TEXT}, {"value", 85, 86, COLUMN_TEXT}};

static const record_column SCAN_METADATA_COLUMNS[] = {
    {"path", 91, 92, COLUMN_TEXT}, {"kind", 93, 94, COLUMN_TEXT},
    {"accession", 95, 96, COLUMN_OPTIONAL_TEXT, 97}, {"name", 98, 99, COLUMN_TEXT},
    {"value", 100, 101, COLUMN_OPTIONAL_TEXT, 102}, {"unit", 103, 104, COLUMN_OPTIONAL_TEXT, 105},
    {"unit_accession", 106, 107, COLUMN_OPTIONAL_TEXT, 108}};

static const record_column *columns_for_kind(uint16_t kind, int *count)
{
  switch (kind)
  {
  case QUANTION_PAYLOAD_PEAKS:
    *count = sizeof(PEAK_COLUMNS) / sizeof(PEAK_COLUMNS[0]);
    return PEAK_COLUMNS;
  case QUANTION_PAYLOAD_FEATURES:
    *count = sizeof(FEATURE_COLUMNS) / sizeof(FEATURE_COLUMNS[0]);
    return FEATURE_COLUMNS;
  case QUANTION_PAYLOAD_FIT_RESULT:
    *count = sizeof(FIT_COLUMNS) / sizeof(FIT_COLUMNS[0]);
    return FIT_COLUMNS;
  case QUANTION_PAYLOAD_CONSENSUS_FEATURES:
    *count = sizeof(CONSENSUS_COLUMNS) / sizeof(CONSENSUS_COLUMNS[0]);
    return CONSENSUS_COLUMNS;
  case QUANTION_PAYLOAD_WARP_TARGETS:
    *count = sizeof(WARP_TARGET_COLUMNS) / sizeof(WARP_TARGET_COLUMNS[0]);
    return WARP_TARGET_COLUMNS;
  case QUANTION_PAYLOAD_MODEL_METADATA:
    *count = sizeof(MODEL_METADATA_COLUMNS) / sizeof(MODEL_METADATA_COLUMNS[0]);
    return MODEL_METADATA_COLUMNS;
  case QUANTION_PAYLOAD_SCAN_METADATA:
    *count = sizeof(SCAN_METADATA_COLUMNS) / sizeof(SCAN_METADATA_COLUMNS[0]);
    return SCAN_METADATA_COLUMNS;
  default:
    *count = 0;
    return NULL;
  }
}

static SEXP count_column(const bridge_view *bridge, uint32_t section_id, R_xlen_t rows)
{
  const bridge_section *section = find_section(bridge, section_id);
  if (section->byte_length / 4 != (uint64_t)rows)
    error("quantion bridge: section %u does not match the record count", section_id);
  SEXP column = PROTECT(Rf_allocVector(REALSXP, rows));
  for (R_xlen_t index = 0; index < rows; index++)
    REAL(column)[index] = (double)read_u32(bridge->raw, section->byte_offset + (uint64_t)index * 4);
  UNPROTECT(1);
  return column;
}

static SEXP text_column(const bridge_view *bridge, uint32_t starts_id, uint32_t bytes_id,
                        R_xlen_t rows)
{
  const bridge_section *starts = find_section(bridge, starts_id);
  const bridge_section *blob = find_section(bridge, bytes_id);
  if (starts->byte_length / 8 != (uint64_t)rows + 1)
    error("quantion bridge: text starts do not match the record count");

  SEXP column = PROTECT(Rf_allocVector(STRSXP, rows));
  uint64_t previous = 0;
  for (R_xlen_t index = 0; index < rows; index++)
  {
    uint64_t from = read_u64(bridge->raw, starts->byte_offset + (uint64_t)index * 8);
    uint64_t to = read_u64(bridge->raw, starts->byte_offset + (uint64_t)(index + 1) * 8);
    if (from < previous || to < from || to > blob->byte_length)
      error("quantion bridge: text starts run past the blob");
    previous = from;
    SET_STRING_ELT(column, index,
                   Rf_mkCharLenCE((const char *)(bridge->raw + blob->byte_offset + from),
                                  (int)(to - from), CE_UTF8));
  }
  UNPROTECT(1);
  return column;
}

static void mark_missing(SEXP column, const bridge_view *bridge, uint32_t missing_id, R_xlen_t rows)
{
  const bridge_section *section = find_section(bridge, missing_id);
  if (section->byte_length != (uint64_t)rows)
    error("quantion bridge: section %u does not match the record count", missing_id);
  for (R_xlen_t index = 0; index < rows; index++)
    if (bridge->raw[section->byte_offset + (uint64_t)index])
      SET_STRING_ELT(column, index, NA_STRING);
}

static SEXP records_bridge_build(void *data)
{
  struct scans_bridge_data *held = (struct scans_bridge_data *)data;
  bridge_view bridge;
  read_bridge_view(&bridge, held->ptr, held->len);

  int column_count = 0;
  const record_column *spec = columns_for_kind(bridge.payload_kind, &column_count);
  if (spec == NULL)
    error("quantion bridge: payload kind %u is not a record table", bridge.payload_kind);

  R_xlen_t rows = (R_xlen_t)bridge.record_count;
  SEXP columns = PROTECT(Rf_allocVector(VECSXP, column_count));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, column_count));
  for (int index = 0; index < column_count; index++)
  {
    SEXP column;
    if (spec[index].kind == COLUMN_NUMBER)
      column = number_column(&bridge, spec[index].id, rows);
    else if (spec[index].kind == COLUMN_COUNT)
      column = count_column(&bridge, spec[index].id, rows);
    else
      column = text_column(&bridge, spec[index].id, spec[index].second_id, rows);
    SET_VECTOR_ELT(columns, index, column);
    if (spec[index].kind == COLUMN_OPTIONAL_TEXT)
      mark_missing(column, &bridge, spec[index].missing_id, rows);
    SET_STRING_ELT(names, index, Rf_mkChar(spec[index].name));
  }
  set_data_frame(columns, names, rows);
  UNPROTECT(2);
  return columns;
}

static SEXP list_get(SEXP lst, const char *name)
{
  if (TYPEOF(lst) != VECSXP)
    return R_NilValue;
  SEXP names = Rf_getAttrib(lst, R_NamesSymbol);
  if (TYPEOF(names) != STRSXP)
    return R_NilValue;
  R_xlen_t n = XLENGTH(lst);
  for (R_xlen_t i = 0; i < n; i++)
  {
    SEXP nm = STRING_ELT(names, i);
    if (nm == R_NilValue)
      continue;
    if (strcmp(CHAR(nm), name) == 0)
      return VECTOR_ELT(lst, i);
  }
  return R_NilValue;
}

typedef struct
{
  uint64_t offset;
  uint64_t length;
  uint8_t *bytes;
} held_range;

typedef struct
{
  held_range *ranges;
  size_t count;
  size_t room;
} range_store;

static range_store *store_new(void)
{
  range_store *store = (range_store *)calloc(1, sizeof(range_store));
  return store;
}

static void store_free(range_store *store)
{
  if (store == NULL)
    return;
  for (size_t index = 0; index < store->count; index++)
    free(store->ranges[index].bytes);
  free(store->ranges);
  free(store);
}

static int store_add(range_store *store, uint64_t offset, const uint8_t *bytes, uint64_t length)
{
  if (store->count == store->room)
  {
    size_t room = store->room == 0 ? 8 : store->room * 2;
    held_range *grown = (held_range *)realloc(store->ranges, room * sizeof(held_range));
    if (grown == NULL)
      return -1;
    store->ranges = grown;
    store->room = room;
  }
  uint8_t *copy = (uint8_t *)malloc(length > 0 ? (size_t)length : 1);
  if (copy == NULL)
    return -1;
  memcpy(copy, bytes, (size_t)length);
  store->ranges[store->count].offset = offset;
  store->ranges[store->count].length = length;
  store->ranges[store->count].bytes = copy;
  store->count++;
  return 0;
}

static int32_t store_serve(void *context, uint64_t offset, uint64_t length, uint8_t *dest)
{
  range_store *store = (range_store *)context;
  if (store == NULL)
    return -1;
  for (size_t index = 0; index < store->count; index++)
  {
    held_range *range = &store->ranges[index];
    if (offset < range->offset)
      continue;
    uint64_t inside = offset - range->offset;
    if (inside > range->length || length > range->length - inside)
      continue;
    memcpy(dest, range->bytes + inside, (size_t)length);
    return 0;
  }
  return -1;
}

static void finalize_store(SEXP ptr)
{
  if (R_ExternalPtrAddr(ptr))
  {
    store_free((range_store *)R_ExternalPtrAddr(ptr));
    R_ClearExternalPtr(ptr);
  }
}


#define QUERY_RANGE_COLUMNS 2

static SEXP take_records(Buf *out)
{
  SEXP cont = PROTECT(R_MakeUnwindCont());
  struct scans_bridge_data held = {out->ptr, out->len};
  SEXP result = R_UnwindProtect(records_bridge_build, &held, scans_bridge_free, &held, cont);
  UNPROTECT(1);
  return result;
}

static SEXP take_numbers(Buf *first, Buf *second, int as_raw)
{
  SEXP cont = PROTECT(R_MakeUnwindCont());
  struct buf_copy_data held = {first->ptr, first->len, second ? second->ptr : NULL, second ? second->len : 0, as_raw};
  SEXP result = R_UnwindProtect(buf_copy_build, &held, buf_copy_free, &held, cont);
  UNPROTECT(1);
  return result;
}

static SEXP take_ranges(Buf *out)
{
  SEXP cont = PROTECT(R_MakeUnwindCont());
  struct buf_copy_data held = {out->ptr, out->len, NULL, 0, 0};
  SEXP result = R_UnwindProtect(ranges_build, &held, buf_copy_free, &held, cont);
  UNPROTECT(1);
  return result;
}

static const uint32_t SCAN_DETAIL_SECTIONS[9] = {
    QUANTION_SECTION_MS_LEVEL,
    QUANTION_SECTION_SELECTED_ION_MZ,
    QUANTION_SECTION_BASE_PEAK_MZ,
    QUANTION_SECTION_BASE_PEAK_INT,
    QUANTION_SECTION_TOTAL_ION_CURRENT,
    QUANTION_SECTION_POLARITY,
    QUANTION_SECTION_POSITION_X,
    QUANTION_SECTION_POSITION_Y,
    QUANTION_SECTION_POSITION_Z,
};

static const char *SCAN_DETAIL_NAMES[9] = {
    "level", "precursor_mz", "base_peak_mz", "base_peak_intensity",
    "total_ion_current", "polarity", "x", "y", "z"};

static SEXP scans_build(void *data)
{
  struct scans_bridge_data *held = (struct scans_bridge_data *)data;
  bridge_view bridge;
  read_bridge_view(&bridge, held->ptr, held->len);
  if (bridge.payload_kind != QUANTION_PAYLOAD_SCANS)
    error("quantion bridge: expected a scans payload");

  R_xlen_t count = (R_xlen_t)bridge.record_count;
  const bridge_section *starts = find_section(&bridge, QUANTION_SECTION_POINT_STARTS);
  if (starts->byte_length / 8 != (uint64_t)count + 1)
    error("quantion bridge: point starts do not match the scan count");
  const bridge_section *mz = find_section(&bridge, QUANTION_SECTION_MZ);
  const bridge_section *intensity = find_section(&bridge, QUANTION_SECTION_INTENSITY);
  if (intensity->byte_length != mz->byte_length)
    error("quantion bridge: intensity length does not match m/z length");
  const unsigned char *starts_at = bridge.raw + starts->byte_offset;

  int column_count = 4 + 9;
  SEXP columns = PROTECT(Rf_allocVector(VECSXP, column_count));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, column_count));
  SET_VECTOR_ELT(columns, 0, number_column(&bridge, QUANTION_SECTION_SCAN_INDEX, count));
  SET_STRING_ELT(names, 0, Rf_mkChar("index"));
  SET_VECTOR_ELT(columns, 1, number_column(&bridge, QUANTION_SECTION_RT, count));
  SET_STRING_ELT(names, 1, Rf_mkChar("rt"));
  SET_VECTOR_ELT(columns, 2, point_column(&bridge, QUANTION_SECTION_MZ, starts_at, count));
  SET_STRING_ELT(names, 2, Rf_mkChar("mz"));
  SET_VECTOR_ELT(columns, 3, point_column(&bridge, QUANTION_SECTION_INTENSITY, starts_at, count));
  SET_STRING_ELT(names, 3, Rf_mkChar("intensity"));
  for (int index = 0; index < 9; index++)
  {
    SET_VECTOR_ELT(columns, 4 + index, number_column(&bridge, SCAN_DETAIL_SECTIONS[index], count));
    SET_STRING_ELT(names, 4 + index, Rf_mkChar(SCAN_DETAIL_NAMES[index]));
  }
  set_data_frame(columns, names, count);
  UNPROTECT(2);
  return columns;
}

static SEXP scan_arrays_build(void *data)
{
  struct scans_bridge_data *held = (struct scans_bridge_data *)data;
  SEXP scans = PROTECT(scans_build(data));
  bridge_view bridge;
  read_bridge_view(&bridge, held->ptr, held->len);
  const bridge_section *starts = find_section(&bridge, QUANTION_SECTION_SCAN_ARRAY_STARTS);
  const bridge_section *values = find_section(&bridge, QUANTION_SECTION_SCAN_ARRAY_VALUES);
  R_xlen_t start_count = (R_xlen_t)(starts->byte_length / 8);
  SEXP array_starts = PROTECT(Rf_allocVector(REALSXP, start_count));
  const unsigned char *starts_at = bridge.raw + starts->byte_offset;
  for (R_xlen_t index = 0; index < start_count; index++)
  {
    uint64_t start;
    memcpy(&start, starts_at + (size_t)index * 8, 8);
    REAL(array_starts)[index] = (double)start;
  }
  R_xlen_t value_count = (R_xlen_t)(values->byte_length / 8);
  SEXP array_values = PROTECT(Rf_allocVector(REALSXP, value_count));
  if (value_count > 0)
    memcpy(REAL(array_values), bridge.raw + values->byte_offset, (size_t)value_count * 8);
  SEXP result = PROTECT(Rf_allocVector(VECSXP, 3));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, 3));
  SET_VECTOR_ELT(result, 0, scans);
  SET_STRING_ELT(names, 0, Rf_mkChar("scans"));
  SET_VECTOR_ELT(result, 1, array_starts);
  SET_STRING_ELT(names, 1, Rf_mkChar("starts"));
  SET_VECTOR_ELT(result, 2, array_values);
  SET_STRING_ELT(names, 2, Rf_mkChar("values"));
  Rf_setAttrib(result, R_NamesSymbol, names);
  UNPROTECT(5);
  return result;
}

static SEXP image_build(void *data)
{
  struct scans_bridge_data *held = (struct scans_bridge_data *)data;
  bridge_view bridge;
  read_bridge_view(&bridge, held->ptr, held->len);
  if (bridge.payload_kind != QUANTION_PAYLOAD_ION_IMAGE)
    error("quantion bridge: expected an ion image payload");
  const bridge_section *shape = find_section(&bridge, QUANTION_SECTION_IMAGE_SHAPE);
  const bridge_section *values = find_section(&bridge, QUANTION_SECTION_IMAGE_DATA);
  const bridge_section *counts = find_section(&bridge, QUANTION_SECTION_IMAGE_COUNTS);
  if (shape->byte_length != 6 * 4)
    error("quantion bridge: the image shape must have six values");

  const char *shape_names[6] = {"width", "height", "min_x", "min_y", "min_z", "max_z"};
  SEXP image = PROTECT(Rf_allocVector(VECSXP, 8));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, 8));
  for (int index = 0; index < 6; index++)
  {
    SET_VECTOR_ELT(image, index, Rf_ScalarInteger((int)read_u32(bridge.raw, shape->byte_offset + (uint64_t)index * 4)));
    SET_STRING_ELT(names, index, Rf_mkChar(shape_names[index]));
  }
  R_xlen_t value_count = (R_xlen_t)(values->byte_length / 8);
  SEXP image_values = PROTECT(Rf_allocVector(REALSXP, value_count));
  if (value_count > 0)
    memcpy(REAL(image_values), bridge.raw + values->byte_offset, (size_t)value_count * 8);
  SET_VECTOR_ELT(image, 6, image_values);
  SET_STRING_ELT(names, 6, Rf_mkChar("data"));
  R_xlen_t count_total = (R_xlen_t)(counts->byte_length / 4);
  SEXP scan_counts = PROTECT(Rf_allocVector(INTSXP, count_total));
  for (R_xlen_t index = 0; index < count_total; index++)
    INTEGER(scan_counts)[index] = (int)read_u32(bridge.raw, counts->byte_offset + (uint64_t)index * 4);
  SET_VECTOR_ELT(image, 7, scan_counts);
  SET_STRING_ELT(names, 7, Rf_mkChar("counts"));
  Rf_setAttrib(image, R_NamesSymbol, names);
  UNPROTECT(4);
  return image;
}

static SEXP take_with(SEXP (*build)(void *), Buf *out)
{
  SEXP cont = PROTECT(R_MakeUnwindCont());
  struct scans_bridge_data held = {out->ptr, out->len};
  SEXP result = R_UnwindProtect(build, &held, scans_bridge_free, &held, cont);
  UNPROTECT(1);
  return result;
}

static SEXP sample_tag(void)
{
  return Rf_install("quantion_sample");
}

static SEXP model_tag(void)
{
  return Rf_install("quantion_model");
}

static SEXP warp_model_tag(void)
{
  return Rf_install("quantion_warp_model");
}

static void finalize_sample(SEXP pointer)
{
  if (R_ExternalPtrAddr(pointer))
  {
    free_mzml((ParsedFile *)R_ExternalPtrAddr(pointer));
    R_ClearExternalPtr(pointer);
  }
}

static SEXP wrap_sample(ParsedFile *handle, SEXP keep)
{
  SEXP pointer = PROTECT(R_MakeExternalPtr(handle, sample_tag(), keep));
  R_RegisterCFinalizerEx(pointer, finalize_sample, TRUE);
  UNPROTECT(1);
  return pointer;
}

static ParsedFile *get_sample(SEXP pointer)
{
  if (TYPEOF(pointer) != EXTPTRSXP || R_ExternalPtrTag(pointer) != sample_tag())
    error("quantion: expected a sample from parse_ion or parse_mzml");
  return (ParsedFile *)R_ExternalPtrAddr(pointer);
}

static const char *get_text_or_null(SEXP value)
{
  if (value == R_NilValue)
    return NULL;
  return Rf_translateCharUTF8(STRING_ELT(value, 0));
}

static uint8_t get_time_unit(SEXP time_unit)
{
  return strcmp(CHAR(STRING_ELT(time_unit, 0)), "SECONDS") == 0 ? 1 : 0;
}

static int get_shape(SEXP shape)
{
  return strcmp(CHAR(STRING_ELT(shape, 0)), "GAUSSIAN") == 0 ? 0 : 1;
}

static double get_number(SEXP list, const char *name)
{
  SEXP value = list_get(list, name);
  return value == R_NilValue ? NAN : asReal(value);
}

static int get_whole(SEXP list, const char *name)
{
  return (int)get_number(list, name);
}

static int get_flag(SEXP list, const char *name)
{
  return asLogical(list_get(list, name)) == TRUE ? 1 : 0;
}

static CPeakOptions get_peak_options(SEXP options)
{
  CPeakOptions peak_options;
  memset(&peak_options, 0, sizeof(peak_options));
  peak_options.min_integral = get_number(options, "min_integral");
  peak_options.min_intensity = get_number(options, "min_intensity");
  peak_options.min_peak_width_points = get_whole(options, "min_peak_width_points");
  peak_options.noise = get_number(options, "noise");
  peak_options.auto_noise = get_flag(options, "auto_noise");
  peak_options.auto_baseline = get_flag(options, "auto_baseline");
  peak_options.lambda = get_number(options, "lambda");
  peak_options.max_iterations = get_whole(options, "max_iterations");
  peak_options.allow_overlap = get_flag(options, "allow_overlap");
  peak_options.min_snr = get_number(options, "min_snr");
  peak_options.min_r2 = get_number(options, "min_r2");
  peak_options.shape = get_shape(list_get(options, "shape"));
  peak_options.kernel_size = get_whole(options, "kernel_size");
  return peak_options;
}

typedef struct
{
  uint32_t *offsets;
  uint32_t *lengths;
  unsigned char *bytes;
  size_t size;
} packed_ids;

static packed_ids pack_ids(SEXP ids, R_xlen_t count)
{
  size_t room = (size_t)(count > 0 ? count : 1);
  packed_ids packed = {(uint32_t *)R_alloc(room, sizeof(uint32_t)),
                       (uint32_t *)R_alloc(room, sizeof(uint32_t)), NULL, 0};
  const char **texts = (const char **)R_alloc(room, sizeof(char *));
  for (R_xlen_t index = 0; index < count; index++)
  {
    SEXP id = ids == R_NilValue ? NA_STRING : STRING_ELT(ids, index);
    texts[index] = id == NA_STRING ? "" : Rf_translateCharUTF8(id);
    packed.size += strlen(texts[index]);
  }
  packed.bytes = (unsigned char *)R_alloc(packed.size > 0 ? packed.size : 1, 1);
  size_t cursor = 0;
  for (R_xlen_t index = 0; index < count; index++)
  {
    size_t length = strlen(texts[index]);
    packed.offsets[index] = (uint32_t)cursor;
    packed.lengths[index] = (uint32_t)length;
    memcpy(packed.bytes + cursor, texts[index], length);
    cursor += length;
  }
  return packed;
}

SEXP C_dispose_sample(SEXP pointer)
{
  get_sample(pointer);
  finalize_sample(pointer);
  return R_NilValue;
}

SEXP C_parse_mzml(SEXP data)
{
  check_bound();
  ParsedFile *handle = NULL;
  check_code(parse_mzml((const uint8_t *)RAW(data), (size_t)XLENGTH(data), &handle));
  return wrap_sample(handle, R_NilValue);
}

SEXP C_parse_mzml_path(SEXP path)
{
  check_bound();
  ParsedFile *handle = NULL;
  check_code(parse_mzml_path(get_text_or_null(path), &handle));
  return wrap_sample(handle, R_NilValue);
}

SEXP C_parse_ion(SEXP data, SEXP max_cache_size)
{
  check_bound();
  ParsedFile *handle = NULL;
  check_code(parse_bin((const uint8_t *)RAW(data), (size_t)XLENGTH(data), (size_t)asReal(max_cache_size), &handle));
  return wrap_sample(handle, R_NilValue);
}

SEXP C_parse_ion_path(SEXP path, SEXP max_cache_size)
{
  check_bound();
  ParsedFile *handle = NULL;
  check_code(parse_ion_path(get_text_or_null(path), (size_t)asReal(max_cache_size), &handle));
  return wrap_sample(handle, R_NilValue);
}

SEXP C_parse_ion_source(SEXP store_pointer, SEXP max_cache_size)
{
  check_bound();
  range_store *store = (range_store *)R_ExternalPtrAddr(store_pointer);
  if (store == NULL)
    error("quantion: the range store is not valid");
  ParsedFile *handle = NULL;
  check_code(parse_ion_source(store_serve, store, (size_t)asReal(max_cache_size), &handle));
  return wrap_sample(handle, store_pointer);
}

SEXP C_plan_open(SEXP header)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_open((const uint8_t *)RAW(header), (size_t)XLENGTH(header), &out));
  return take_ranges(&out);
}

SEXP C_store_new(void)
{
  range_store *store = store_new();
  if (store == NULL)
    error("quantion: cannot make a range store");
  SEXP pointer = PROTECT(R_MakeExternalPtr(store, R_NilValue, R_NilValue));
  R_RegisterCFinalizerEx(pointer, finalize_store, TRUE);
  UNPROTECT(1);
  return pointer;
}

SEXP C_store_add(SEXP store_pointer, SEXP offset, SEXP bytes)
{
  range_store *store = (range_store *)R_ExternalPtrAddr(store_pointer);
  if (store == NULL)
    error("quantion: the range store is not valid");
  if (store_add(store, (uint64_t)asReal(offset), (const uint8_t *)RAW(bytes), (uint64_t)XLENGTH(bytes)) != 0)
    error("quantion: cannot hold that range");
  return R_NilValue;
}

SEXP C_write_ion(SEXP file, SEXP out_path, SEXP overwrite, SEXP level, SEXP f32_compress, SEXP block_size,
                 SEXP mz_window, SEXP headers)
{
  check_bound();
  ParsedFile *handle = get_sample(file);
  size_t headers_size = 0;
  R_xlen_t header_count = XLENGTH(headers);
  for (R_xlen_t index = 0; index < header_count; index++)
    headers_size += strlen(Rf_translateCharUTF8(STRING_ELT(headers, index))) + 1;
  uint8_t *header_bytes = (uint8_t *)R_alloc(headers_size > 0 ? headers_size : 1, 1);
  size_t cursor = 0;
  for (R_xlen_t index = 0; index < header_count; index++)
  {
    const char *text = Rf_translateCharUTF8(STRING_ELT(headers, index));
    size_t length = strlen(text);
    memcpy(header_bytes + cursor, text, length);
    header_bytes[cursor + length] = 0;
    cursor += length + 1;
  }
  Buf out = (Buf){0};
  check_code(write_ion(handle, get_text_or_null(out_path), (uint8_t)asLogical(overwrite), (uint8_t)asInteger(level),
                       (uint8_t)asLogical(f32_compress), (uint32_t)asReal(block_size), (uint32_t)asReal(mz_window),
                       header_bytes, headers_size, &out));
  if (out_path != R_NilValue && out.ptr == NULL)
    return R_NilValue;
  return take_numbers(&out, NULL, 1);
}

SEXP C_write_mzml(SEXP file, SEXP out_path, SEXP overwrite)
{
  check_bound();
  ParsedFile *handle = get_sample(file);
  Buf out = (Buf){0};
  check_code(write_mzml(handle, get_text_or_null(out_path), (uint8_t)asLogical(overwrite), &out));
  if (out_path != R_NilValue)
    return R_NilValue;
  return take_numbers(&out, NULL, 1);
}

SEXP C_get_scans(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(get_scans(get_sample(file), (uint8_t)asInteger(query_type), asReal(from), asReal(to),
                       (uint8_t)asInteger(level), get_time_unit(time_unit), &out));
  return take_with(scans_build, &out);
}

SEXP C_plan_scans(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_scans(get_sample(file), (uint8_t)asInteger(query_type), asReal(from), asReal(to),
                        (uint8_t)asInteger(level), get_time_unit(time_unit), &out));
  return take_ranges(&out);
}

SEXP C_get_scan_metadata(SEXP file, SEXP index, SEXP tags)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(get_scan_metadata(get_sample(file), (size_t)asReal(index), CHAR(STRING_ELT(tags, 0)), &out));
  return take_records(&out);
}

SEXP C_plan_scan_metadata(SEXP file, SEXP index)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_scan_metadata(get_sample(file), (size_t)asReal(index), &out));
  return take_ranges(&out);
}

static uint32_t *get_accessions(SEXP accessions)
{
  R_xlen_t count = XLENGTH(accessions);
  uint32_t *numbers = (uint32_t *)R_alloc((size_t)count, sizeof(uint32_t));
  for (R_xlen_t index = 0; index < count; index++)
    numbers[index] = (uint32_t)REAL(accessions)[index];
  return numbers;
}

SEXP C_get_scans_with_arrays(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit, SEXP accessions)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(get_scans_with_arrays(get_sample(file), (uint8_t)asInteger(query_type), asReal(from), asReal(to),
                                   (uint8_t)asInteger(level), get_time_unit(time_unit),
                                   get_accessions(accessions), (size_t)XLENGTH(accessions), &out));
  return take_with(scan_arrays_build, &out);
}

SEXP C_plan_scans_with_arrays(SEXP file, SEXP query_type, SEXP from, SEXP to, SEXP level, SEXP time_unit, SEXP accessions)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_scans_with_arrays(get_sample(file), (uint8_t)asInteger(query_type), asReal(from), asReal(to),
                                    (uint8_t)asInteger(level), get_time_unit(time_unit),
                                    get_accessions(accessions), (size_t)XLENGTH(accessions), &out));
  return take_ranges(&out);
}

SEXP C_calculate_eic(SEXP file, SEXP mz, SEXP eic)
{
  check_bound();
  Buf x = (Buf){0}, y = (Buf){0};
  check_code(calculate_eic(get_sample(file), asReal(mz), get_number(eic, "from"), get_number(eic, "to"),
                           get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                           get_time_unit(list_get(eic, "time_unit")), &x, &y));
  return take_numbers(&x, &y, 0);
}

SEXP C_plan_eic(SEXP file, SEXP mz, SEXP eic)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_eic(get_sample(file), asReal(mz), get_number(eic, "from"), get_number(eic, "to"),
                      get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                      get_time_unit(list_get(eic, "time_unit")), &out));
  return take_ranges(&out);
}

SEXP C_get_ion_image(SEXP file, SEXP mz, SEXP tolerance, SEXP level)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(get_ion_image(get_sample(file), asReal(mz), asReal(tolerance), (uint8_t)asInteger(level), &out));
  return take_with(image_build, &out);
}

SEXP C_plan_image(SEXP file, SEXP mz, SEXP tolerance, SEXP level)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_image(get_sample(file), asReal(mz), asReal(tolerance), (uint8_t)asInteger(level), &out));
  return take_ranges(&out);
}

SEXP C_find_peaks(SEXP x, SEXP y, SEXP options)
{
  check_bound();
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(find_peaks(REAL(x), REAL(y), (size_t)XLENGTH(x), &peak_options, &out));
  return take_records(&out);
}

SEXP C_get_peak(SEXP x, SEXP y, SEXP rt, SEXP range, SEXP options)
{
  check_bound();
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(get_peak(REAL(x), REAL(y), (size_t)XLENGTH(x), asReal(rt), asReal(range), &peak_options, &out));
  return take_records(&out);
}

SEXP C_fit_peak(SEXP x, SEXP y, SEXP rt, SEXP shape, SEXP intensity)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(fit_peak(REAL(x), REAL(y), (size_t)XLENGTH(x), asReal(rt), get_shape(shape), asReal(intensity), &out));
  return take_records(&out);
}

SEXP C_draw_peaks(SEXP heights, SEXP centers, SEXP fwhms, SEXP tails, SEXP from, SEXP to, SEXP points)
{
  check_bound();
  Buf x = (Buf){0}, y = (Buf){0};
  check_code(draw_peaks(REAL(heights), REAL(centers), REAL(fwhms), REAL(tails), (size_t)XLENGTH(heights),
                        asReal(from), asReal(to), (size_t)asReal(points), &x, &y));
  return take_numbers(&x, &y, 0);
}

SEXP C_find_noise_level(SEXP y)
{
  check_bound();
  R_xlen_t count = XLENGTH(y);
  float *values = (float *)R_alloc((size_t)(count > 0 ? count : 1), sizeof(float));
  for (R_xlen_t index = 0; index < count; index++)
    values[index] = (float)REAL(y)[index];
  size_t width = 0;
  double intensity = 0.0;
  check_code(find_noise_level(values, (size_t)count, &width, &intensity));
  SEXP result = PROTECT(Rf_allocVector(VECSXP, 2));
  SET_VECTOR_ELT(result, 0, Rf_ScalarInteger((int)width));
  SET_VECTOR_ELT(result, 1, Rf_ScalarReal(intensity));
  SEXP names = PROTECT(Rf_allocVector(STRSXP, 2));
  SET_STRING_ELT(names, 0, Rf_mkChar("width"));
  SET_STRING_ELT(names, 1, Rf_mkChar("intensity"));
  Rf_setAttrib(result, R_NamesSymbol, names);
  UNPROTECT(2);
  return result;
}

SEXP C_calculate_baseline(SEXP y, SEXP lambda, SEXP max_iterations)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(calculate_baseline(REAL(y), (size_t)XLENGTH(y), asReal(lambda), (size_t)asReal(max_iterations), &out));
  return take_numbers(&out, NULL, 0);
}

SEXP C_find_targets(SEXP file, SEXP rts, SEXP mzs, SEXP ranges, SEXP ids, SEXP eic, SEXP options, SEXP threads)
{
  check_bound();
  R_xlen_t count = XLENGTH(rts);
  packed_ids packed = pack_ids(ids, count);
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(find_targets(get_sample(file), REAL(rts), REAL(mzs), REAL(ranges), packed.offsets, packed.lengths,
                          packed.bytes, packed.size, (size_t)count, get_number(eic, "from"), get_number(eic, "to"),
                          get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                          get_time_unit(list_get(eic, "time_unit")), &peak_options, (size_t)asReal(threads), &out));
  return take_records(&out);
}

SEXP C_plan_find_targets(SEXP file, SEXP mzs, SEXP eic)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_find_targets(get_sample(file), REAL(mzs), (size_t)XLENGTH(mzs), get_number(eic, "from"),
                               get_number(eic, "to"), get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                               get_time_unit(list_get(eic, "time_unit")), &out));
  return take_ranges(&out);
}

SEXP C_find_transitions(SEXP file, SEXP precursors, SEXP products, SEXP rts, SEXP ranges, SEXP ids, SEXP options, SEXP threads)
{
  check_bound();
  R_xlen_t count = XLENGTH(rts);
  packed_ids packed = pack_ids(ids, count);
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(find_transitions(get_sample(file), REAL(precursors), REAL(products), REAL(rts), REAL(ranges),
                              packed.offsets, packed.lengths, packed.bytes, packed.size, (size_t)count,
                              &peak_options, (size_t)asReal(threads), &out));
  return take_records(&out);
}

SEXP C_find_features(SEXP file, SEXP eic, SEXP grid, SEXP options, SEXP threads)
{
  check_bound();
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(find_features(get_sample(file), get_number(eic, "from"), get_number(eic, "to"),
                           get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                           get_time_unit(list_get(eic, "time_unit")), get_number(grid, "start"),
                           get_number(grid, "end"), get_number(grid, "step_size"), &peak_options,
                           (size_t)asReal(threads), &out));
  return take_records(&out);
}

SEXP C_plan_find_features(SEXP file, SEXP eic, SEXP grid)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(plan_find_features(get_sample(file), get_number(eic, "from"), get_number(eic, "to"),
                                get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                                get_time_unit(list_get(eic, "time_unit")), get_number(grid, "start"),
                                get_number(grid, "end"), get_number(grid, "step_size"), &out));
  return take_ranges(&out);
}

SEXP C_deisotope_features(SEXP features, SEXP count, SEXP rt_tolerance, SEXP mz_tolerance)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(deisotope_features(REAL(features), (size_t)asReal(count), asReal(rt_tolerance), asReal(mz_tolerance), &out));
  return take_records(&out);
}

SEXP C_match_features(SEXP directory, SEXP eic, SEXP grid, SEXP grouping, SEXP options, SEXP threads)
{
  check_bound();
  CPeakOptions peak_options = get_peak_options(options);
  Buf out = (Buf){0};
  check_code(match_features(get_text_or_null(directory), get_number(eic, "from"), get_number(eic, "to"),
                            get_number(eic, "ppm_tolerance"), get_number(eic, "mz_tolerance"),
                            get_time_unit(list_get(eic, "time_unit")), get_number(grid, "start"),
                            get_number(grid, "end"), get_number(grid, "step_size"),
                            get_number(grouping, "ppm_tolerance"), get_number(grouping, "mz_tolerance"),
                            get_number(grouping, "rt_tolerance"), get_number(grouping, "min_frequency"),
                            &peak_options, (size_t)asReal(threads), &out));
  return take_records(&out);
}

static void finalize_model(SEXP pointer)
{
  if (R_ExternalPtrAddr(pointer))
  {
    model_free((ModelHandle *)R_ExternalPtrAddr(pointer));
    R_ClearExternalPtr(pointer);
  }
}

static void finalize_warp_model(SEXP pointer)
{
  if (R_ExternalPtrAddr(pointer))
  {
    warp_model_free((WarpModelHandle *)R_ExternalPtrAddr(pointer));
    R_ClearExternalPtr(pointer);
  }
}

static SEXP wrap_model(ModelHandle *model)
{
  SEXP pointer = PROTECT(R_MakeExternalPtr(model, model_tag(), R_NilValue));
  R_RegisterCFinalizerEx(pointer, finalize_model, TRUE);
  UNPROTECT(1);
  return pointer;
}

static SEXP wrap_warp_model(WarpModelHandle *warp)
{
  SEXP pointer = PROTECT(R_MakeExternalPtr(warp, warp_model_tag(), R_NilValue));
  R_RegisterCFinalizerEx(pointer, finalize_warp_model, TRUE);
  UNPROTECT(1);
  return pointer;
}

static ModelHandle *get_model(SEXP pointer)
{
  if (TYPEOF(pointer) != EXTPTRSXP || R_ExternalPtrTag(pointer) != model_tag())
    error("quantion: expected a model from load_model");
  ModelHandle *model = (ModelHandle *)R_ExternalPtrAddr(pointer);
  if (!model)
    error("quantion: use of a disposed model");
  return model;
}

static WarpModelHandle *get_warp_model(SEXP pointer)
{
  if (TYPEOF(pointer) != EXTPTRSXP || R_ExternalPtrTag(pointer) != warp_model_tag())
    error("quantion: expected a warp model from load_warp");
  WarpModelHandle *warp = (WarpModelHandle *)R_ExternalPtrAddr(pointer);
  if (!warp)
    error("quantion: use of a disposed warp model");
  return warp;
}

static void check_message_code(int code, Buf *message)
{
  if (code == 0)
    return;
  if (message->ptr)
    free_(message->ptr, message->len);
  message->ptr = NULL;
  stop_with_last_error(code);
}

SEXP C_model_load_path(SEXP path)
{
  check_bound();
  const char *text = get_text_or_null(path);
  ModelHandle *model = NULL;
  Buf message = (Buf){0};
  check_message_code(model_load_path((const uint8_t *)text, strlen(text), &model, &message), &message);
  return wrap_model(model);
}

SEXP C_model_load_bytes(SEXP bytes)
{
  check_bound();
  ModelHandle *model = NULL;
  Buf message = (Buf){0};
  check_message_code(model_load_bytes((const uint8_t *)RAW(bytes), (size_t)XLENGTH(bytes), &model, &message), &message);
  return wrap_model(model);
}

SEXP C_dispose_model(SEXP pointer)
{
  if (TYPEOF(pointer) != EXTPTRSXP || R_ExternalPtrTag(pointer) != model_tag())
    error("quantion: expected a model from load_model");
  finalize_model(pointer);
  return R_NilValue;
}

SEXP C_model_metadata(SEXP pointer)
{
  check_bound();
  Buf out = (Buf){0};
  check_code(model_metadata(get_model(pointer), &out));
  return take_records(&out);
}

SEXP C_warp_model_load(SEXP pointer)
{
  check_bound();
  WarpModelHandle *warp = NULL;
  Buf message = (Buf){0};
  check_message_code(warp_model_load(get_model(pointer), &warp, &message), &message);
  return wrap_warp_model(warp);
}

SEXP C_dispose_warp_model(SEXP pointer)
{
  if (TYPEOF(pointer) != EXTPTRSXP || R_ExternalPtrTag(pointer) != warp_model_tag())
    error("quantion: expected a warp model from load_warp");
  finalize_warp_model(pointer);
  return R_NilValue;
}

SEXP C_predict_rts(SEXP warp, SEXP file, SEXP mzs, SEXP rts)
{
  check_bound();
  Buf out = (Buf){0};
  check_message_code(predict_rts_from_file(get_warp_model(warp), get_sample(file), REAL(mzs), REAL(rts),
                                           (size_t)XLENGTH(mzs), &out), &out);
  return take_numbers(&out, NULL, 0);
}

SEXP C_plan_predict_rts(SEXP warp, SEXP file, SEXP mzs, SEXP rts)
{
  check_bound();
  Buf out = (Buf){0};
  check_message_code(plan_predict_rts(get_warp_model(warp), get_sample(file), REAL(mzs), REAL(rts),
                                      (size_t)XLENGTH(mzs), &out), &out);
  return take_ranges(&out);
}

SEXP C_calibrate_rts(SEXP warp, SEXP samples, SEXP list_lengths, SEXP ids, SEXP mzs, SEXP rts)
{
  check_bound();
  R_xlen_t file_count = XLENGTH(samples);
  R_xlen_t list_count = XLENGTH(list_lengths);
  R_xlen_t count = XLENGTH(mzs);
  const ParsedFile **files = (const ParsedFile **)R_alloc((size_t)(file_count > 0 ? file_count : 1), sizeof(ParsedFile *));
  size_t *lengths = (size_t *)R_alloc((size_t)(list_count > 0 ? list_count : 1), sizeof(size_t));
  for (R_xlen_t index = 0; index < file_count; index++)
    files[index] = get_sample(VECTOR_ELT(samples, index));
  for (R_xlen_t index = 0; index < list_count; index++)
    lengths[index] = (size_t)REAL(list_lengths)[index];
  packed_ids packed = pack_ids(ids, count);
  Buf out = (Buf){0};
  check_message_code(calibrate_rts(get_warp_model(warp), files, (size_t)file_count, lengths, (size_t)list_count, packed.offsets,
                                   packed.lengths, packed.bytes, packed.size, REAL(mzs), REAL(rts),
                                   (size_t)count, &out), &out);
  return take_records(&out);
}
