.ERROR_CODES <- c(
  invalid_argument = 1L,
  missing_argument = 2L,
  unknown_option = 3L,
  remote_file = 4L,
  file_not_found = 5L,
  parse_failed = 6L,
  model_mismatch = 7L,
  file_exists = 8L,
  upload_failed = 9L,
  internal = 99L
)

.signal_error <- function(kind, function_name, argument, message) {
  condition <- structure(
    list(
      message = paste0(function_name, ": ", message),
      call = NULL,
      kind = kind,
      code = unname(.ERROR_CODES[kind]),
      `function` = function_name,
      argument = argument
    ),
    class = c("quantion_error", "error", "condition")
  )
  stop(condition)
}

.stop_with_rust_error <- function(text, code) {
  parts <- strsplit(text, "\x1f", fixed = TRUE)[[1]]
  if (length(parts) < 5) {
    .signal_error("internal", "quantion", NULL, sprintf("the call failed with code %d", code))
  }
  argument <- if (nzchar(parts[4])) parts[4] else NULL
  .signal_error(parts[2], parts[3], argument, paste(parts[-(1:4)], collapse = "\x1f"))
}

.invalid <- function(function_name, argument, message) {
  .signal_error("invalid_argument", function_name, argument, paste(argument, message))
}

.missing <- function(function_name, argument) {
  .signal_error("missing_argument", function_name, argument, paste(argument, "is required"))
}

.check_number <- function(function_name, argument, value) {
  if (!is.numeric(value) || length(value) != 1 || is.na(value))
    .invalid(function_name, argument, "must be one number")
  as.numeric(value)
}

.check_optional_number <- function(function_name, argument, value) {
  if (length(value) == 1 && is.na(value)) return(NA_real_)
  .check_number(function_name, argument, value)
}

.check_flag <- function(function_name, argument, value) {
  if (!is.logical(value) || length(value) != 1 || is.na(value))
    .invalid(function_name, argument, "must be TRUE or FALSE")
  value
}

.check_text <- function(function_name, argument, value) {
  if (!is.character(value) || length(value) != 1 || is.na(value) || !nzchar(value))
    .invalid(function_name, argument, "must be one non-empty string")
  value
}

.check_choice <- function(function_name, argument, value, choices) {
  if (!is.character(value) || length(value) != 1 || is.na(value))
    .invalid(function_name, argument, paste("must be one of", paste(choices, collapse = ", ")))
  tryCatch(
    match.arg(value, choices),
    error = function(error) .invalid(function_name, argument,
                                     sprintf("must be one of %s, got %s", paste(choices, collapse = ", "), value))
  )
}

.check_options <- function(function_name, argument, value, class_name, helper) {
  if (!inherits(value, class_name))
    .invalid(function_name, argument, sprintf("must be built with %s()", helper))
  value
}

.check_sample <- function(function_name, file) {
  if (!inherits(file, "quantion_sample"))
    .invalid(function_name, "file", "must be a sample from parse_ion or parse_mzml")
  file
}

.check_trace <- function(function_name, x, y) {
  if (!is.numeric(x)) .invalid(function_name, "x", "must be numeric")
  if (!is.numeric(y)) .invalid(function_name, "y", "must be numeric")
  list(x = as.numeric(x), y = as.numeric(y))
}

.as_options <- function(class_name, values) {
  structure(values, class = c(class_name, "quantion_options"))
}

print.quantion_options <- function(x, ...) {
  cat("<", class(x)[1], ">\n", sep = "")
  for (name in names(x)) {
    value <- x[[name]]
    shown <- if (inherits(value, "quantion_options")) paste0("<", class(value)[1], ">")
             else if (is.null(value)) "NULL"
             else if (is.function(value)) "<function>"
             else if (is.list(value)) sprintf("list of %d", length(value))
             else paste(format(value), collapse = ", ")
    cat("  ", name, ": ", shown, "\n", sep = "")
  }
  invisible(x)
}

header <- function(name, value) {
  name <- .check_text("header", "name", name)
  if (!is.character(value) || length(value) != 1 || is.na(value))
    .invalid("header", "value", "must be one string")
  .as_options("quantion_header", list(name = name, value = value))
}

parse_ion_options <- function(max_cache_size = 0) {
  .as_options("parse_ion_options", list(
    max_cache_size = .check_number("parse_ion_options", "max_cache_size", max_cache_size)
  ))
}

.LARGEST_U32 <- 4294967295

.check_whole_u32 <- function(function_name, argument, value, smallest) {
  if (!is.numeric(value) || length(value) != 1 || !is.finite(value) || value != floor(value) ||
      value < smallest || value > .LARGEST_U32)
    .invalid(function_name, argument, sprintf("must be a whole number from %d to %.0f", smallest, .LARGEST_U32))
  as.numeric(value)
}

write_ion_options <- function(out = NULL, overwrite = FALSE, level = 12L, f32_compress = FALSE, headers = list(),
                              block_size = 8L, mz_window = 20L) {
  function_name <- "write_ion_options"
  if (!is.null(out)) out <- .check_text(function_name, "out", out)
  if (inherits(headers, "quantion_header")) headers <- list(headers)
  if (!is.list(headers)) .invalid(function_name, "headers", "must be a list of header()")
  for (index in seq_along(headers)) {
    if (!inherits(headers[[index]], "quantion_header"))
      .invalid(function_name, sprintf("headers[[%d]]", index), "must be built with header()")
  }
  .as_options("write_ion_options", list(
    out = out,
    overwrite = .check_flag(function_name, "overwrite", overwrite),
    level = .check_number(function_name, "level", level),
    f32_compress = .check_flag(function_name, "f32_compress", f32_compress),
    headers = headers,
    block_size = .check_whole_u32(function_name, "block_size", block_size, 1),
    mz_window = .check_whole_u32(function_name, "mz_window", mz_window, 0)
  ))
}

write_mzml_options <- function(out = NULL, overwrite = FALSE) {
  function_name <- "write_mzml_options"
  if (!is.null(out)) out <- .check_text(function_name, "out", out)
  .as_options("write_mzml_options", list(
    out = out,
    overwrite = .check_flag(function_name, "overwrite", overwrite)
  ))
}

.DEFAULT_SCAN_ARRAYS <- list(
  list(key = "mz", accession = "MS:1000514"),
  list(key = "intensity", accession = "MS:1000515")
)
.MAX_ACCESSION <- 4294967295
.LARGEST_WHOLE_NUMBER <- 2^53 - 1
.RESERVED_SCAN_KEYS <- c("index", "rt", "level", "precursor_mz", "precursorMz", "base_peak_mz", "basePeakMz", "base_peak_intensity", "basePeakIntensity", "total_ion_current", "totalIonCurrent", "polarity", "x", "y", "z")

.check_scan_array <- function(function_name, array, seen_keys) {
  if (!is.list(array) || is.null(names(array)))
    .invalid(function_name, "arrays", "must hold lists with key and accession")
  for (field in names(array)) {
    if (!(field %in% c("key", "accession"))) .invalid(function_name, paste0("arrays.", field), "is not an option")
  }
  key <- array$key
  accession <- array$accession
  if (!is.character(key) || length(key) != 1 || is.na(key) || !nzchar(key))
    .invalid(function_name, "arrays.key", "must not be empty")
  if (key %in% .RESERVED_SCAN_KEYS)
    .invalid(function_name, "arrays.key", sprintf("cannot be %s: a scan already has that field", key))
  if (!is.character(accession) || length(accession) != 1 || is.na(accession) ||
      !grepl("^MS:[0-9]+$", accession) || as.numeric(sub("MS:", "", accession)) > .MAX_ACCESSION)
    .invalid(function_name, "arrays.accession", "must look like MS:1000514")
  for (pair in .DEFAULT_SCAN_ARRAYS) {
    if ((key == pair$key) != (accession == pair$accession))
      .invalid(function_name, "arrays", sprintf("%s and %s must go together", pair$key, pair$accession))
  }
  if (key %in% seen_keys) .invalid(function_name, "arrays.key", sprintf("repeats %s", key))
  list(key = key, accession = accession)
}

.check_scan_arrays <- function(function_name, arrays) {
  if (!is.list(arrays) || length(arrays) == 0)
    .invalid(function_name, "arrays", "must have at least one array")
  seen_keys <- character()
  checked <- list()
  for (array in arrays) {
    found <- .check_scan_array(function_name, array, seen_keys)
    seen_keys <- c(seen_keys, found$key)
    checked[[length(checked) + 1L]] <- found
  }
  checked
}

get_scans_options <- function(level = 1L, time_unit = "MINUTES",
                              arrays = list(list(key = "mz", accession = "MS:1000514"),
                                            list(key = "intensity", accession = "MS:1000515"))) {
  function_name <- "get_scans_options"
  .as_options("get_scans_options", list(
    level = .check_number(function_name, "level", level),
    time_unit = .check_choice(function_name, "time_unit", time_unit, c("MINUTES", "SECONDS")),
    arrays = .check_scan_arrays(function_name, arrays)
  ))
}

.SCAN_METADATA_TAGS <- c(
  "spectrum", "referenceableParamGroupRef", "spectrumDescription", "scanList", "scan", "scanWindowList",
  "scanWindow", "precursorList", "precursor", "isolationWindow", "selectedIonList", "selectedIon",
  "activation", "productList", "product", "binaryDataArrayList", "binaryDataArray"
)

.check_scan_metadata_tags <- function(function_name, tags) {
  if (!is.character(tags) || anyNA(tags) || !all(tags %in% .SCAN_METADATA_TAGS))
    .invalid(function_name, "tags", sprintf("must be tag names from: %s", paste(.SCAN_METADATA_TAGS, collapse = ", ")))
  tags
}

get_scan_metadata_options <- function(tags = character(0)) {
  function_name <- "get_scan_metadata_options"
  .as_options("get_scan_metadata_options", list(
    tags = .check_scan_metadata_tags(function_name, tags)
  ))
}

calculate_eic_options <- function(from = 0, to = Inf, ppm_tolerance = 20, mz_tolerance = 0.005, time_unit = "MINUTES") {
  function_name <- "calculate_eic_options"
  .as_options("calculate_eic_options", list(
    from = .check_number(function_name, "from", from),
    to = .check_number(function_name, "to", to),
    ppm_tolerance = .check_number(function_name, "ppm_tolerance", ppm_tolerance),
    mz_tolerance = .check_number(function_name, "mz_tolerance", mz_tolerance),
    time_unit = .check_choice(function_name, "time_unit", time_unit, c("MINUTES", "SECONDS"))
  ))
}

get_ion_image_options <- function(tolerance = 0.125, level = 1L, on_progress = NULL) {
  function_name <- "get_ion_image_options"
  if (!is.null(on_progress) && !is.function(on_progress))
    .invalid(function_name, "on_progress", "must be a function or NULL")
  .as_options("get_ion_image_options", list(
    tolerance = .check_number(function_name, "tolerance", tolerance),
    level = .check_number(function_name, "level", level),
    on_progress = on_progress
  ))
}

find_peaks_options <- function(min_integral = NA, min_intensity = 500, min_peak_width_points = 3L,
                               noise = NA, auto_noise = TRUE, auto_baseline = TRUE, lambda = 20,
                               max_iterations = 4L, allow_overlap = FALSE, min_snr = 2, min_r2 = 0,
                               shape = "EMG", kernel_size = 0L) {
  function_name <- "find_peaks_options"
  .as_options("find_peaks_options", list(
    min_integral = .check_optional_number(function_name, "min_integral", min_integral),
    min_intensity = .check_number(function_name, "min_intensity", min_intensity),
    min_peak_width_points = .check_number(function_name, "min_peak_width_points", min_peak_width_points),
    noise = .check_optional_number(function_name, "noise", noise),
    auto_noise = .check_flag(function_name, "auto_noise", auto_noise),
    auto_baseline = .check_flag(function_name, "auto_baseline", auto_baseline),
    lambda = .check_number(function_name, "lambda", lambda),
    max_iterations = .check_number(function_name, "max_iterations", max_iterations),
    allow_overlap = .check_flag(function_name, "allow_overlap", allow_overlap),
    min_snr = .check_number(function_name, "min_snr", min_snr),
    min_r2 = .check_number(function_name, "min_r2", min_r2),
    shape = .check_choice(function_name, "shape", shape, c("EMG", "GAUSSIAN")),
    kernel_size = .check_number(function_name, "kernel_size", kernel_size)
  ))
}

get_peak_options <- function(range = 0, find_peaks = find_peaks_options()) {
  function_name <- "get_peak_options"
  .as_options("get_peak_options", list(
    range = .check_number(function_name, "range", range),
    find_peaks = .check_options(function_name, "find_peaks", find_peaks, "find_peaks_options", "find_peaks_options")
  ))
}

fit_peak_options <- function(shape = "EMG", intensity = 0) {
  function_name <- "fit_peak_options"
  .as_options("fit_peak_options", list(
    shape = .check_choice(function_name, "shape", shape, c("EMG", "GAUSSIAN")),
    intensity = .check_number(function_name, "intensity", intensity)
  ))
}

draw_peaks_options <- function(from = 0, to = 0, points = 1000L) {
  function_name <- "draw_peaks_options"
  .as_options("draw_peaks_options", list(
    from = .check_number(function_name, "from", from),
    to = .check_number(function_name, "to", to),
    points = .check_number(function_name, "points", points)
  ))
}

calculate_baseline_options <- function(lambda = 20, max_iterations = 4L) {
  function_name <- "calculate_baseline_options"
  .as_options("calculate_baseline_options", list(
    lambda = .check_number(function_name, "lambda", lambda),
    max_iterations = .check_number(function_name, "max_iterations", max_iterations)
  ))
}

grid_options <- function(start = 40, end = 1000, step_size = 0.005) {
  function_name <- "grid_options"
  .as_options("grid_options", list(
    start = .check_number(function_name, "start", start),
    end = .check_number(function_name, "end", end),
    step_size = .check_number(function_name, "step_size", step_size)
  ))
}

grouping_options <- function(ppm_tolerance = 5, mz_tolerance = 0.0025, rt_tolerance = 0.05, min_frequency = 0) {
  function_name <- "grouping_options"
  .as_options("grouping_options", list(
    ppm_tolerance = .check_number(function_name, "ppm_tolerance", ppm_tolerance),
    mz_tolerance = .check_number(function_name, "mz_tolerance", mz_tolerance),
    rt_tolerance = .check_number(function_name, "rt_tolerance", rt_tolerance),
    min_frequency = .check_number(function_name, "min_frequency", min_frequency)
  ))
}

.check_threads <- function(function_name, threads) {
  threads <- .check_number(function_name, "threads", threads)
  if (threads < 1 || threads != floor(threads))
    .invalid(function_name, "threads", sprintf("must be a whole number of 1 or more, got %s", format(threads)))
  threads
}

find_targets_options <- function(eic = calculate_eic_options(), find_peaks = find_peaks_options(), threads = 1L) {
  function_name <- "find_targets_options"
  .as_options("find_targets_options", list(
    eic = .check_options(function_name, "eic", eic, "calculate_eic_options", "calculate_eic_options"),
    find_peaks = .check_options(function_name, "find_peaks", find_peaks, "find_peaks_options", "find_peaks_options"),
    threads = .check_threads(function_name, threads)
  ))
}

find_transitions_options <- function(find_peaks = find_peaks_options(), threads = 1L) {
  function_name <- "find_transitions_options"
  .as_options("find_transitions_options", list(
    find_peaks = .check_options(function_name, "find_peaks", find_peaks, "find_peaks_options", "find_peaks_options"),
    threads = .check_threads(function_name, threads)
  ))
}

find_features_options <- function(eic = calculate_eic_options(), grid = grid_options(),
                                  find_peaks = find_peaks_options(), threads = 1L) {
  function_name <- "find_features_options"
  .as_options("find_features_options", list(
    eic = .check_options(function_name, "eic", eic, "calculate_eic_options", "calculate_eic_options"),
    grid = .check_options(function_name, "grid", grid, "grid_options", "grid_options"),
    find_peaks = .check_options(function_name, "find_peaks", find_peaks, "find_peaks_options", "find_peaks_options"),
    threads = .check_threads(function_name, threads)
  ))
}

match_features_options <- function(eic = calculate_eic_options(), grid = grid_options(), grouping = grouping_options(),
                                   find_peaks = find_peaks_options(), threads = 1L) {
  function_name <- "match_features_options"
  .as_options("match_features_options", list(
    eic = .check_options(function_name, "eic", eic, "calculate_eic_options", "calculate_eic_options"),
    grid = .check_options(function_name, "grid", grid, "grid_options", "grid_options"),
    grouping = .check_options(function_name, "grouping", grouping, "grouping_options", "grouping_options"),
    find_peaks = .check_options(function_name, "find_peaks", find_peaks, "find_peaks_options", "find_peaks_options"),
    threads = .check_threads(function_name, threads)
  ))
}

deisotope_features_options <- function(rt_tolerance = 0.02, mz_tolerance = 0.005) {
  function_name <- "deisotope_features_options"
  .as_options("deisotope_features_options", list(
    rt_tolerance = .check_number(function_name, "rt_tolerance", rt_tolerance),
    mz_tolerance = .check_number(function_name, "mz_tolerance", mz_tolerance)
  ))
}

.PEAK_COLUMNS <- c("rt", "from", "to", "intensity", "integral", "n_points")
.FEATURE_COLUMNS <- c("mz", "rt", "from", "to", "intensity", "integral", "n_points")

.as_table <- function(rows, columns) {
  rows <- as.data.frame(rows)[, columns, drop = FALSE]
  rownames(rows) <- NULL
  rows
}

.is_url <- function(text) {
  is.character(text) && length(text) == 1 && !is.na(text) && grepl("^https?://", text, ignore.case = TRUE)
}

.remote_source <- function(file) attr(file, "quantion_source")

.as_sample <- function(handle) {
  class(handle) <- "quantion_sample"
  handle
}

parse_ion <- function(source, options = parse_ion_options()) {
  function_name <- "parse_ion"
  options <- .check_options(function_name, "options", options, "parse_ion_options", "parse_ion_options")
  cache <- options$max_cache_size
  if (is.raw(source)) return(.as_sample(.Call("C_parse_ion", source, cache, PACKAGE = "quantion")))
  source <- .check_text(function_name, "source", source)
  if (.is_url(source)) return(.parse_ion_remote(source, cache))
  .as_sample(.Call("C_parse_ion_path", path.expand(source), cache, PACKAGE = "quantion"))
}

parse_mzml <- function(source) {
  function_name <- "parse_mzml"
  if (is.raw(source)) return(.as_sample(.Call("C_parse_mzml", source, PACKAGE = "quantion")))
  source <- .check_text(function_name, "source", source)
  .as_sample(.Call("C_parse_mzml_path", path.expand(source), PACKAGE = "quantion"))
}

.get_out <- function(out) {
  if (is.null(out) || .is_url(out)) out else path.expand(out)
}

write_ion <- function(file, options = write_ion_options()) {
  function_name <- "write_ion"
  file <- .check_sample(function_name, file)
  options <- .check_options(function_name, "options", options, "write_ion_options", "write_ion_options")
  out <- .get_out(options$out)
  headers <- as.character(unlist(lapply(options$headers, function(item) c(item$name, item$value)), use.names = FALSE))
  bytes <- .Call("C_write_ion", file, out, options$overwrite, as.integer(options$level),
                 options$f32_compress, options$block_size, options$mz_window, headers, PACKAGE = "quantion")
  if (!is.null(out) && .is_url(out)) {
    .upload_ion(out, options$headers, bytes)
    return(invisible(NULL))
  }
  if (is.null(out)) bytes else invisible(NULL)
}

.upload_ion <- function(url, headers, bytes) {
  handle <- curl::new_handle(customrequest = "PUT", postfields = bytes)
  if (length(headers)) {
    fields <- vapply(headers, function(item) item$value, character(1))
    names(fields) <- vapply(headers, function(item) trimws(item$name), character(1))
    curl::handle_setheaders(handle, .list = as.list(fields))
  }
  response <- tryCatch(
    curl::curl_fetch_memory(url, handle = handle),
    error = function(error) .signal_error("upload_failed", "write_ion", NULL, paste("PUT failed:", conditionMessage(error)))
  )
  if (response$status_code >= 400)
    .signal_error("upload_failed", "write_ion", NULL, paste("PUT returned", response$status_code))
  invisible(NULL)
}

write_mzml <- function(file, options = write_mzml_options()) {
  function_name <- "write_mzml"
  file <- .check_sample(function_name, file)
  options <- .check_options(function_name, "options", options, "write_mzml_options", "write_mzml_options")
  out <- .get_out(options$out)
  bytes <- .Call("C_write_mzml", file, out, options$overwrite, PACKAGE = "quantion")
  if (is.null(out)) bytes else invisible(NULL)
}

.POSITION_ANY <- 0
.POSITION_RANGE <- 1
.POSITION_CLOSEST <- 2

.get_position <- function(function_name, value, name) {
  if (is.null(value)) return(c(.POSITION_ANY, NaN, NaN))
  if (!is.numeric(value) || !(length(value) %in% c(1, 2)) || anyNA(value))
    .invalid(function_name, name, "must be one number or c(from, to)")
  value <- as.numeric(value)
  if (length(value) == 2) return(c(.POSITION_RANGE, value[1], value[2]))
  c(.POSITION_CLOSEST, value, NaN)
}

.add_scan_arrays <- function(found, keys) {
  scans <- found$scans
  scan_count <- nrow(scans)
  key_count <- length(keys)
  if (length(found$starts) != scan_count * key_count + 1)
    stop("quantion bridge: array starts do not match the scans and arrays")
  read_values <- function(scan_index, key_index) {
    position <- (scan_index - 1) * key_count + key_index
    first <- found$starts[position] + 1
    last <- found$starts[position + 1]
    if (last < first) numeric() else found$values[first:last]
  }
  for (key_index in seq_along(keys)) {
    scans[[keys[key_index]]] <- lapply(seq_len(scan_count), read_values, key_index = key_index)
  }
  scans
}

get_scans <- function(file, rt = NULL, mz = NULL, options = get_scans_options()) {
  function_name <- "get_scans"
  file <- .check_sample(function_name, file)
  options <- .check_options(function_name, "options", options, "get_scans_options", "get_scans_options")
  if (is.null(rt) && is.null(mz)) .invalid(function_name, "rt", "or mz must be given")
  query <- c(.get_position(function_name, rt, "rt"), .get_position(function_name, mz, "mz"))
  level <- as.integer(options$level)
  source <- .remote_source(file)
  if (!identical(options$arrays, .DEFAULT_SCAN_ARRAYS)) {
    keys <- vapply(options$arrays, function(array) array$key, character(1))
    accessions <- vapply(options$arrays, function(array) as.numeric(sub("MS:", "", array$accession)), numeric(1))
    if (!is.null(source)) {
      .prefetch(source, .Call("C_plan_scans_with_arrays", file, query, level,
                              options$time_unit, accessions, PACKAGE = "quantion"))
    }
    found <- .Call("C_get_scans_with_arrays", file, query, level,
                   options$time_unit, accessions, PACKAGE = "quantion")
    return(.add_scan_arrays(found, keys))
  }
  if (!is.null(source)) {
    .prefetch(source, .Call("C_plan_scans", file, query, level,
                            options$time_unit, PACKAGE = "quantion"))
  }
  .Call("C_get_scans", file, query, level, options$time_unit,
        PACKAGE = "quantion")
}

.check_scan_index <- function(function_name, index) {
  if (!is.numeric(index) || length(index) != 1 || !is.finite(index) || index < 0 || index != floor(index))
    .invalid(function_name, "index", "must be a whole number 0 or more")
  if (index > .LARGEST_WHOLE_NUMBER)
    .invalid(function_name, "index", sprintf("must be less than the scan count, got %.0f", index))
  as.numeric(index)
}

get_scan_metadata <- function(file, index, options = get_scan_metadata_options()) {
  function_name <- "get_scan_metadata"
  file <- .check_sample(function_name, file)
  options <- .check_options(function_name, "options", options, "get_scan_metadata_options", "get_scan_metadata_options")
  index <- .check_scan_index(function_name, index)
  source <- .remote_source(file)
  if (!is.null(source)) .prefetch(source, .Call("C_plan_scan_metadata", file, index, PACKAGE = "quantion"))
  tags <- paste(options$tags, collapse = "\x1f")
  .Call("C_get_scan_metadata", file, index, tags, PACKAGE = "quantion")
}

calculate_eic <- function(file, mz, options = calculate_eic_options()) {
  function_name <- "calculate_eic"
  file <- .check_sample(function_name, file)
  mz <- .check_number(function_name, "mz", mz)
  options <- .check_options(function_name, "options", options, "calculate_eic_options", "calculate_eic_options")
  source <- .remote_source(file)
  if (!is.null(source)) .prefetch(source, .Call("C_plan_eic", file, mz, options, PACKAGE = "quantion"))
  .Call("C_calculate_eic", file, mz, options, PACKAGE = "quantion")
}

get_ion_image <- function(file, mz, options = get_ion_image_options()) {
  function_name <- "get_ion_image"
  file <- .check_sample(function_name, file)
  mz <- .check_number(function_name, "mz", mz)
  options <- .check_options(function_name, "options", options, "get_ion_image_options", "get_ion_image_options")
  level <- as.integer(options$level)
  source <- .remote_source(file)
  if (!is.null(source)) {
    wanted <- .Call("C_plan_image", file, mz, options$tolerance, level, PACKAGE = "quantion")
    .prefetch(source, wanted, options$on_progress)
  }
  .Call("C_get_ion_image", file, mz, options$tolerance, level, PACKAGE = "quantion")
}

find_peaks <- function(x, y, options = find_peaks_options()) {
  function_name <- "find_peaks"
  trace <- .check_trace(function_name, x, y)
  options <- .check_options(function_name, "options", options, "find_peaks_options", "find_peaks_options")
  .as_table(.Call("C_find_peaks", trace$x, trace$y, options, PACKAGE = "quantion"), .PEAK_COLUMNS)
}

get_peak <- function(x, y, rt, options = get_peak_options()) {
  function_name <- "get_peak"
  trace <- .check_trace(function_name, x, y)
  rt <- .check_number(function_name, "rt", rt)
  options <- .check_options(function_name, "options", options, "get_peak_options", "get_peak_options")
  rows <- .Call("C_get_peak", trace$x, trace$y, rt, options$range, options$find_peaks, PACKAGE = "quantion")
  as.list(.as_table(rows, .PEAK_COLUMNS)[1, ])
}

fit_peak <- function(x, y, rt, options = fit_peak_options()) {
  function_name <- "fit_peak"
  trace <- .check_trace(function_name, x, y)
  rt <- .check_number(function_name, "rt", rt)
  options <- .check_options(function_name, "options", options, "fit_peak_options", "fit_peak_options")
  rows <- .Call("C_fit_peak", trace$x, trace$y, rt, options$shape, options$intensity, PACKAGE = "quantion")
  if (nrow(rows) == 0) return(NULL)
  as.list(rows[1, ])
}

.get_peak_value <- function(function_name, peak, index, name, required) {
  argument <- sprintf("params[[%d]]$%s", index, name)
  value <- peak[[name]]
  if (is.null(value)) {
    if (!required) return(0)
    .missing(function_name, argument)
  }
  .check_number(function_name, argument, value)
}

draw_peaks <- function(params, options = draw_peaks_options()) {
  function_name <- "draw_peaks"
  options <- .check_options(function_name, "options", options, "draw_peaks_options", "draw_peaks_options")
  if (!is.list(params)) .invalid(function_name, "params", "must be a list of PeakParameters lists")
  if (!is.null(params$center) && !is.list(params$center)) params <- list(params)
  for (index in seq_along(params)) .check_names(function_name, params[[index]], .PEAK_NAMES, sprintf("params[[%d]]", index))
  values <- function(name, required) {
    vapply(seq_along(params), function(index)
      .get_peak_value(function_name, params[[index]], index, name, required), numeric(1))
  }
  .Call("C_draw_peaks", values("height", TRUE), values("center", TRUE), values("fwhm", TRUE),
        values("tail", TRUE), options$from, options$to, options$points, PACKAGE = "quantion")
}

find_noise_level <- function(y) {
  if (!is.numeric(y)) .invalid("find_noise_level", "y", "must be numeric")
  .Call("C_find_noise_level", as.numeric(y), PACKAGE = "quantion")
}

calculate_baseline <- function(y, options = calculate_baseline_options()) {
  function_name <- "calculate_baseline"
  if (!is.numeric(y)) .invalid(function_name, "y", "must be numeric")
  options <- .check_options(function_name, "options", options, "calculate_baseline_options", "calculate_baseline_options")
  .Call("C_calculate_baseline", as.numeric(y), options$lambda, options$max_iterations, PACKAGE = "quantion")
}

.get_column <- function(function_name, table, name, required, empty, argument = "targets") {
  if (name %in% names(table)) return(table[[name]])
  if (required) .missing(function_name, sprintf("%s$%s", argument, name))
  rep(empty, nrow(table))
}

.TARGET_NAMES <- c("mz", "rt", "id", "range")
.TRANSITION_NAMES <- c("precursor", "product", "rt", "range", "id")
.PEAK_NAMES <- c("height", "center", "fwhm", "tail", "r2")

.check_names <- function(function_name, given, allowed, argument) {
  for (name in names(given)) {
    if (!(name %in% allowed)) {
      path <- paste0(argument, "$", name)
      .signal_error("unknown_option", function_name, path,
                    sprintf("%s is not an option; the options are %s", path, paste(allowed, collapse = ", ")))
    }
  }
}

.check_table <- function(function_name, targets, allowed, argument = "targets") {
  if (!is.data.frame(targets)) .invalid(function_name, argument, "must be a data.frame")
  .check_names(function_name, targets, allowed, argument)
  targets
}

find_targets <- function(file, targets, options = find_targets_options()) {
  function_name <- "find_targets"
  file <- .check_sample(function_name, file)
  targets <- .check_table(function_name, targets, .TARGET_NAMES)
  options <- .check_options(function_name, "options", options, "find_targets_options", "find_targets_options")
  mzs <- as.numeric(.get_column(function_name, targets, "mz", TRUE, NA))
  rts <- as.numeric(.get_column(function_name, targets, "rt", TRUE, NA))
  ranges <- as.numeric(.get_column(function_name, targets, "range", FALSE, NA))
  ranges[is.na(ranges)] <- NaN
  ids <- as.character(.get_column(function_name, targets, "id", FALSE, NA))
  source <- .remote_source(file)
  if (!is.null(source)) .prefetch(source, .Call("C_plan_find_targets", file, mzs, options$eic, PACKAGE = "quantion"))
  peaks <- .Call("C_find_targets", file, rts, mzs, ranges, ids, options$eic, options$find_peaks,
                 options$threads, PACKAGE = "quantion")
  .as_table(peaks, .PEAK_COLUMNS)
}

find_transitions <- function(file, targets, options = find_transitions_options()) {
  function_name <- "find_transitions"
  file <- .check_sample(function_name, file)
  targets <- .check_table(function_name, targets, .TRANSITION_NAMES)
  options <- .check_options(function_name, "options", options, "find_transitions_options", "find_transitions_options")
  column <- function(name) as.numeric(.get_column(function_name, targets, name, TRUE, NA))
  ids <- as.character(.get_column(function_name, targets, "id", FALSE, NA))
  peaks <- .Call("C_find_transitions", file, column("precursor"), column("product"), column("rt"),
                 column("range"), ids, options$find_peaks, options$threads, PACKAGE = "quantion")
  .as_table(peaks, .PEAK_COLUMNS)
}

find_features <- function(file, options = find_features_options()) {
  function_name <- "find_features"
  file <- .check_sample(function_name, file)
  options <- .check_options(function_name, "options", options, "find_features_options", "find_features_options")
  source <- .remote_source(file)
  if (!is.null(source)) {
    .prefetch(source, .Call("C_plan_find_features", file, options$eic, options$grid, PACKAGE = "quantion"))
  }
  features <- .Call("C_find_features", file, options$eic, options$grid, options$find_peaks, options$threads,
                    PACKAGE = "quantion")
  .as_table(features, .FEATURE_COLUMNS)
}

match_features <- function(directory, options = match_features_options()) {
  function_name <- "match_features"
  directory <- .check_text(function_name, "directory", directory)
  options <- .check_options(function_name, "options", options, "match_features_options", "match_features_options")
  features <- .Call("C_match_features", path.expand(directory), options$eic, options$grid, options$grouping,
                    options$find_peaks, options$threads, PACKAGE = "quantion")
  .as_table(features, c(.FEATURE_COLUMNS, "frequency"))
}

deisotope_features <- function(features, options = deisotope_features_options()) {
  function_name <- "deisotope_features"
  features <- .check_table(function_name, features, c(.FEATURE_COLUMNS, "frequency"), "features")
  options <- .check_options(function_name, "options", options, "deisotope_features_options", "deisotope_features_options")
  columns <- lapply(.FEATURE_COLUMNS, function(name) as.numeric(.get_column(function_name, features, name, TRUE, NA, "features")))
  frequency <- as.numeric(.get_column(function_name, features, "frequency", FALSE, NA, "features"))
  has_frequency <- any(!is.na(frequency))
  frequency[is.na(frequency)] <- NaN
  values <- as.numeric(t(do.call(cbind, c(columns, list(frequency)))))
  found <- .Call("C_deisotope_features", values, as.numeric(nrow(features)), options$rt_tolerance,
                 options$mz_tolerance, PACKAGE = "quantion")
  .as_table(found, if (has_frequency) c(.FEATURE_COLUMNS, "frequency") else .FEATURE_COLUMNS)
}

dispose <- function(object) {
  if (inherits(object, "quantion_warp_model"))
    return(invisible(.Call("C_dispose_warp_model", object, PACKAGE = "quantion")))
  if (inherits(object, "quantion_model"))
    return(invisible(.Call("C_dispose_model", object, PACKAGE = "quantion")))
  if (inherits(object, "quantion_sample"))
    return(invisible(.Call("C_dispose_sample", object, PACKAGE = "quantion")))
  .invalid("dispose", "object", "must be a sample, a model or a warp model")
}

load_model <- function(source) {
  function_name <- "load_model"
  if (.is_url(source)) {
    destination <- tempfile()
    on.exit(unlink(destination), add = TRUE)
    status <- tryCatch(
      utils::download.file(source, destination, method = "libcurl", quiet = TRUE, mode = "wb"),
      error = function(error) conditionMessage(error)
    )
    if (!identical(status, 0L))
      .signal_error("parse_failed", function_name, NULL,
                    sprintf("the file could not be read: GET %s failed: %s", source, status))
    source <- readBin(destination, "raw", file.info(destination)$size)
  }
  model <- if (is.raw(source)) {
    .Call("C_model_load_bytes", source, PACKAGE = "quantion")
  } else {
    .Call("C_model_load_path", path.expand(.check_text(function_name, "source", source)), PACKAGE = "quantion")
  }
  class(model) <- "quantion_model"
  attr(model, "metadata") <- .Call("C_model_metadata", model, PACKAGE = "quantion")
  model
}

load_warp <- function(model) {
  if (!inherits(model, "quantion_model")) .invalid("load_warp", "model", "must be a model from load_model")
  warp <- .Call("C_warp_model_load", model, PACKAGE = "quantion")
  class(warp) <- "quantion_warp_model"
  attr(warp, "metadata") <- attr(model, "metadata")
  warp
}

.get_metadata <- function(function_name, model) {
  if (!inherits(model, c("quantion_model", "quantion_warp_model")))
    .invalid(function_name, "model", "must be a model from load_model or a warp model from load_warp")
  attr(model, "metadata")
}

get_keys <- function(model) {
  .get_metadata("get_keys", model)$key
}

get_values <- function(model) {
  .get_metadata("get_values", model)$value
}

get_value <- function(model, key) {
  metadata <- .get_metadata("get_value", model)
  key <- .check_text("get_value", "key", key)
  value <- metadata$value[metadata$key == key]
  if (length(value) == 0) NULL else value[[1]]
}

.check_warp <- function(function_name, warp) {
  if (!inherits(warp, "quantion_warp_model")) .invalid(function_name, "warp", "must be a warp model from load_warp")
  warp
}

.prefetch_targets <- function(warp, file, mzs, rts) {
  source <- .remote_source(file)
  if (is.null(source) || length(mzs) == 0) return(invisible(NULL))
  .prefetch(source, .Call("C_plan_predict_rts", warp, file, mzs, rts, PACKAGE = "quantion"))
}

predict_rts <- function(warp, file, targets) {
  function_name <- "predict_rts"
  warp <- .check_warp(function_name, warp)
  file <- .check_sample(function_name, file)
  targets <- .check_table(function_name, targets, .TARGET_NAMES)
  mzs <- as.numeric(.get_column(function_name, targets, "mz", TRUE, NA))
  rts <- as.numeric(.get_column(function_name, targets, "rt", TRUE, NA))
  if (length(mzs) == 0) return(data.frame(rt = numeric(0), uncertainty = numeric(0)))
  .prefetch_targets(warp, file, mzs, rts)
  values <- .Call("C_predict_rts", warp, file, mzs, rts, PACKAGE = "quantion")
  data.frame(rt = values[c(TRUE, FALSE)], uncertainty = values[c(FALSE, TRUE)])
}

calibrate <- function(warp, anchor_samples, target_lists) {
  function_name <- "calibrate"
  warp <- .check_warp(function_name, warp)
  if (!is.list(anchor_samples) || inherits(anchor_samples, "quantion_sample"))
    .invalid(function_name, "anchor_samples", "must be a list of samples")
  if (!is.list(target_lists) || is.data.frame(target_lists))
    .invalid(function_name, "target_lists", "must be a list of data.frames")
  for (index in seq_along(anchor_samples)) .check_sample(function_name, anchor_samples[[index]])
  for (index in seq_along(target_lists)) {
    argument <- sprintf("target_lists[[%d]]", index)
    targets <- .check_table(function_name, target_lists[[index]], .TARGET_NAMES, argument)
    if (index <= length(anchor_samples)) {
      .prefetch_targets(warp, anchor_samples[[index]],
                        as.numeric(.get_column(function_name, targets, "mz", TRUE, NA, argument)),
                        as.numeric(.get_column(function_name, targets, "rt", TRUE, NA, argument)))
    }
  }
  combined <- function(name, required) {
    unlist(lapply(seq_along(target_lists), function(index)
      .get_column(function_name, target_lists[[index]], name, required, NA, sprintf("target_lists[[%d]]", index))),
      use.names = FALSE)
  }
  ids <- as.character(combined("id", FALSE))
  .Call("C_calibrate_rts", warp, anchor_samples, as.numeric(vapply(target_lists, nrow, integer(1))),
        ids, as.numeric(combined("mz", TRUE)), as.numeric(combined("rt", TRUE)), PACKAGE = "quantion")
}

.fetch_range <- function(url, offset, span) {
  destination <- tempfile()
  on.exit(unlink(destination), add = TRUE)
  last <- offset + span - 1
  status <- tryCatch(
    utils::download.file(url, destination, method = "libcurl", quiet = TRUE, mode = "wb",
                         headers = c(Range = sprintf("bytes=%.0f-%.0f", offset, last))),
    error = function(error) conditionMessage(error)
  )
  if (!identical(status, 0L))
    .signal_error("parse_failed", "parse_ion", NULL, sprintf("the file could not be read: GET %s failed", url))
  bytes <- readBin(destination, "raw", file.info(destination)$size)
  if (length(bytes) != span)
    .signal_error("parse_failed", "parse_ion", NULL,
                  sprintf("the server returned %d bytes for a %d byte range; it does not support range requests",
                          length(bytes), span))
  bytes
}

.LARGEST_GAP <- 131072

.gap_for <- function(total) {
  min(.LARGEST_GAP, total / 8)
}

.merge_ranges <- function(ranges, gap = .LARGEST_GAP) {
  wanted <- ranges[ranges[, 2] > 0, , drop = FALSE]
  if (nrow(wanted) == 0) return(wanted)
  wanted <- wanted[order(wanted[, 1]), , drop = FALSE]

  merged <- list(c(wanted[1, 1], wanted[1, 2]))
  for (row in seq_len(nrow(wanted))[-1]) {
    last <- merged[[length(merged)]]
    if (wanted[row, 1] - (last[1] + last[2]) <= gap) {
      end <- max(last[1] + last[2], wanted[row, 1] + wanted[row, 2])
      merged[[length(merged)]] <- c(last[1], end - last[1])
    } else {
      merged[[length(merged) + 1]] <- c(wanted[row, 1], wanted[row, 2])
    }
  }
  do.call(rbind, merged)
}

.already_held <- function(source, offset, span) {
  for (held in source$held) {
    if (offset >= held[1] && offset + span <= held[1] + held[2]) return(TRUE)
  }
  FALSE
}

.held_bytes <- function(source) {
  sum(vapply(source$held, function(held) held[2], numeric(1)))
}

.prefetch <- function(source, ranges, on_progress = NULL) {
  if (nrow(ranges) == 0) return(invisible(NULL))
  wanted <- ranges[!apply(ranges, 1, function(row) .already_held(source, row[1], row[2])), , drop = FALSE]
  if (nrow(wanted) == 0) return(invisible(NULL))

  spans <- .merge_ranges(wanted, .gap_for(source$total))
  total <- sum(spans[, 2])
  fetched <- 0
  for (row in seq_len(nrow(spans))) {
    offset <- spans[row, 1]
    span <- spans[row, 2]
    if (.already_held(source, offset, span)) next
    bytes <- .fetch_range(source$url, offset, span)
    .Call("C_store_add", source$store, offset, bytes, PACKAGE = "quantion")
    source$held[[length(source$held) + 1]] <- c(offset, length(bytes))
    fetched <- fetched + length(bytes)
    if (!is.null(on_progress)) on_progress(fetched, total, .held_bytes(source))
  }
  invisible(NULL)
}

.parse_ion_remote <- function(url, max_cache_size = 0) {
  source <- new.env(parent = emptyenv())
  source$url <- url
  source$store <- .Call("C_store_new", PACKAGE = "quantion")
  source$held <- list()
  source$total <- 0

  header <- .fetch_range(url, 0, 1024)
  .Call("C_store_add", source$store, 0, header, PACKAGE = "quantion")
  source$held[[1]] <- c(0, length(header))

  planned <- .Call("C_plan_open", header, PACKAGE = "quantion")
  source$total <- if (nrow(planned)) max(planned[, 1] + planned[, 2]) else 0
  .prefetch(source, planned)

  handle <- .Call("C_parse_ion_source", source$store, as.numeric(max_cache_size), PACKAGE = "quantion")
  handle <- .as_sample(handle)
  attr(handle, "quantion_source") <- source
  handle
}

.DEFAULT_LIBRARY <- "https://github.com/josoriom/cipher/releases/latest/download/library_positive.ion"
.DEFAULT_MODEL <- "https://github.com/josoriom/cipher/releases/latest/download/model.onnx"
.ANNOTATION_FEATURE_NAMES <- c("mz", "rt", "from", "to", "intensity", "integral", "n_points", "frequency")

annotate_features_options <- function(library = .DEFAULT_LIBRARY, model = .DEFAULT_MODEL, matches = 3L,
                                      ppm_tolerance = 20, mz_tolerance = 0.005, rt_range = 0.3,
                                      min_relative_intensity = 0.02) {
  function_name <- "annotate_features_options"
  .as_options("annotate_features_options", list(
    library = .check_text(function_name, "library", library),
    model = .check_text(function_name, "model", model),
    matches = .check_number(function_name, "matches", matches),
    ppm_tolerance = .check_number(function_name, "ppm_tolerance", ppm_tolerance),
    mz_tolerance = .check_number(function_name, "mz_tolerance", mz_tolerance),
    rt_range = .check_number(function_name, "rt_range", rt_range),
    min_relative_intensity = .check_number(function_name, "min_relative_intensity", min_relative_intensity)
  ))
}

.get_compound_matches <- function(rows) {
  if (nrow(rows) == 0)
    return(data.frame(label = character(), id = character(), database = character(), score = numeric(),
                      cosine = numeric(), stringsAsFactors = FALSE))
  matches <- rows[, c("label", "id", "database", "score", "cosine")]
  matches$mz <- rows$mz
  matches$intensity <- rows$intensity
  matches$metadata <- lapply(seq_len(nrow(rows)), function(index) {
    names <- rows$metadata_names[index]
    if (!nzchar(names)) return(character())
    stats::setNames(strsplit(rows$metadata_values[index], "\x1f", fixed = TRUE)[[1]],
                    strsplit(names, "\x1f", fixed = TRUE)[[1]])
  })
  rownames(matches) <- NULL
  matches
}

.get_annotations <- function(tables) {
  features <- tables[[1]]
  files <- tables[[2]]
  matches <- tables[[3]]
  features$score[is.nan(features$score)] <- NA
  features$cosine[is.nan(features$cosine)] <- NA
  features$files <- lapply(seq_len(nrow(features)) - 1, function(feature) {
    rows <- files[files$feature == feature, c("file", "scan_index"), drop = FALSE]
    rows$scan_index[is.nan(rows$scan_index)] <- NA
    rows$matches <- lapply(rows$file, function(file) {
      .get_compound_matches(matches[matches$feature == feature & matches$file == file, , drop = FALSE])
    })
    rownames(rows) <- NULL
    rows
  })
  features
}

annotate_features <- function(files, features, options = annotate_features_options()) {
  function_name <- "annotate_features"
  options <- .check_options(function_name, "options", options, "annotate_features_options", "annotate_features_options")
  if (!is.list(files) || inherits(files, "quantion_sample"))
    .invalid(function_name, "files", "must be a list of samples")
  for (index in seq_along(files)) .check_sample(function_name, files[[index]])
  features <- .check_table(function_name, features, .ANNOTATION_FEATURE_NAMES, "features")
  mzs <- as.numeric(.get_column(function_name, features, "mz", TRUE, NA, "features"))
  rts <- as.numeric(.get_column(function_name, features, "rt", TRUE, NA, "features"))
  settings <- as.numeric(c(options$matches, options$ppm_tolerance, options$mz_tolerance, options$rt_range,
                           options$min_relative_intensity))
  if (length(mzs) == 0)
    return(.get_annotations(.Call("C_annotate_features", NULL, NULL, files, mzs, rts, settings, PACKAGE = "quantion")))
  library <- parse_ion(options$library)
  model <- load_model(options$model)
  on.exit({
    dispose(library)
    dispose(model)
  }, add = TRUE)
  for (file in files) {
    source <- .remote_source(file)
    if (!is.null(source))
      .prefetch(source, .Call("C_plan_annotate_samples", file, mzs, rts, settings, PACKAGE = "quantion"))
  }
  source <- .remote_source(library)
  if (!is.null(source))
    .prefetch(source, .Call("C_plan_annotate_library", library, model, files, mzs, rts, settings, PACKAGE = "quantion"))
  .get_annotations(.Call("C_annotate_features", library, model, files, mzs, rts, settings, PACKAGE = "quantion"))
}

