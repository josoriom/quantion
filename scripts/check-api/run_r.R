suppressMessages(library(quantion))

FROM_RT <- 0.0
TO_RT <- 5.0
EIC_PPM <- 20.0
EIC_MZ <- 0.005
GRID_START <- 89.0
GRID_END <- 91.0
GRID_STEP <- 0.005
ROI_RANGE <- 0.05
THREADS <- 1L

FEATURE_MIN_INTENSITY <- 500.0
FEATURE_MIN_PEAK_WIDTH_POINTS <- 5L

SELF_CHECK_VALUES <- c(1, 2, 3)

CHANNELS <- list(
  list(
    name = "mz_2", mass = 90.05550,
    min_intensity = 500.0, min_peak_width_points = 5L,
    targets = list(c("met_1", 2.885), c("met_2", 2.552), c("met_3", 2.465))
  ),
  list(
    name = "mz_1", mass = 89.04768,
    min_intensity = 500.0, min_peak_width_points = 5L,
    targets = list(c("met_1", 2.385), c("met_2", 2.18), c("met_3", 2.08))
  )
)

number <- function(value) {
  value <- as.numeric(value)
  if (is.nan(value)) return("nan")
  if (is.infinite(value)) return(if (value > 0) "inf" else "-inf")
  sprintf("%.17g", value)
}

FNV_OFFSET <- 2166136261
FNV_PRIME_LOW <- 403
FNV_PRIME_HIGH_SHIFT <- 16777216

fnv1a <- function(values) {
  hash <- FNV_OFFSET
  for (value in values) {
    for (byte in as.integer(writeBin(as.numeric(value), raw(), size = 8, endian = "little"))) {
      low <- hash %% 256
      hash <- hash - low + bitwXor(low, byte)
      hash <- (hash * FNV_PRIME_LOW + (hash %% 256) * FNV_PRIME_HIGH_SHIFT) %% 4294967296
    }
  }
  sprintf("%04x%04x", as.integer(hash %/% 65536), as.integer(hash %% 65536))
}

peak_lines <- function(prefix, peak, out) {
  out <- shared_peak_lines(prefix, peak, out)
  c(out, sprintf("%s.n_points = %d", prefix, as.integer(peak$n_points)))
}

shared_peak_lines <- function(prefix, peak, out) {
  c(out,
    sprintf("%s.rt = %s", prefix, number(peak$rt)),
    sprintf("%s.from = %s", prefix, number(peak$from)),
    sprintf("%s.to = %s", prefix, number(peak$to)),
    sprintf("%s.integral = %s", prefix, number(peak$integral)),
    sprintf("%s.intensity = %s", prefix, number(peak$intensity)))
}

window_options <- function() {
  calculate_eic_options(from = FROM_RT, to = TO_RT, ppm_tolerance = EIC_PPM, mz_tolerance = EIC_MZ)
}

peak_options <- function(channel, automatic = TRUE) {
  find_peaks_options(min_intensity = channel$min_intensity,
                     min_peak_width_points = channel$min_peak_width_points,
                     auto_noise = automatic, auto_baseline = automatic)
}

feature_peak_options <- function() {
  find_peaks_options(min_intensity = FEATURE_MIN_INTENSITY,
                     min_peak_width_points = FEATURE_MIN_PEAK_WIDTH_POINTS,
                     auto_noise = TRUE, auto_baseline = TRUE)
}

grid <- function() grid_options(start = GRID_START, end = GRID_END, step_size = GRID_STEP)

report_channel <- function(file, transitions, channel, out) {
  name <- channel$name
  eic <- calculate_eic(file, channel$mass, window_options())
  x <- as.numeric(eic$x)
  y <- as.numeric(eic$y)

  noise <- find_noise_level(y)

  out <- c(out,
    sprintf("%s.calculate_eic.points = %d", name, length(x)),
    sprintf("%s.calculate_eic.x.fnv1a = %s", name, fnv1a(x)),
    sprintf("%s.calculate_eic.y.fnv1a = %s", name, fnv1a(y)),
    sprintf("%s.find_noise_level.intensity = %s", name, number(noise$intensity)),
    sprintf("%s.find_noise_level.width = %d", name, noise$width))

  found <- find_peaks(x, y, peak_options(channel))
  out <- c(out, sprintf("%s.find_peaks.count = %d", name, nrow(found)))
  for (index in seq_len(nrow(found))) {
    out <- peak_lines(sprintf("%s.find_peaks[%d]", name, index - 1L), found[index, ], out)
  }

  target <- as.numeric(channel$targets[[3]][2])
  single <- get_peak(x, y, target, get_peak_options(range = ROI_RANGE, find_peaks = peak_options(channel)))
  out <- peak_lines(sprintf("%s.get_peak", name), single, out)

  ids <- vapply(channel$targets, function(t) t[1], character(1))
  rts <- vapply(channel$targets, function(t) as.numeric(t[2]), numeric(1))
  targets <- data.frame(id = ids, rt = rts, mz = channel$mass, range = ROI_RANGE,
                        stringsAsFactors = FALSE)
  rows <- find_targets(file, targets, find_targets_options(
    eic = window_options(), find_peaks = peak_options(channel), threads = THREADS))
  for (index in seq_len(nrow(rows))) {
    out <- shared_peak_lines(
      sprintf("%s.find_targets.%s", name, targets$id[index]), rows[index, ], out)
  }

  chrom_targets <- data.frame(id = ids, precursor = channel$mass, product = channel$mass,
                              rt = rts, range = ROI_RANGE, stringsAsFactors = FALSE)
  chrom_rows <- find_transitions(transitions, chrom_targets, find_transitions_options(
    find_peaks = peak_options(channel, automatic = FALSE), threads = THREADS))
  for (index in seq_len(nrow(chrom_rows))) {
    prefix <- sprintf("%s.find_transitions.%s", name, ids[index])
    row <- chrom_rows[index, ]
    out <- c(out,
      sprintf("%s.rt = %s", prefix, number(row$rt)),
      sprintf("%s.from = %s", prefix, number(row$from)),
      sprintf("%s.to = %s", prefix, number(row$to)),
      sprintf("%s.intensity = %s", prefix, number(row$intensity)),
      sprintf("%s.area = %s", prefix, number(row$integral)))
  }
  out
}

feature_lines <- function(label, rows, out, with_points) {
  out <- c(out, sprintf("%s.count = %d", label, nrow(rows)))
  for (index in seq_len(nrow(rows))) {
    name <- sprintf("%s[%d]", label, index - 1L)
    row <- rows[index, ]
    out <- c(out,
      sprintf("%s.mz = %s", name, number(row$mz)),
      sprintf("%s.rt = %s", name, number(row$rt)),
      sprintf("%s.from = %s", name, number(row$from)),
      sprintf("%s.to = %s", name, number(row$to)),
      sprintf("%s.integral = %s", name, number(row$integral)),
      sprintf("%s.intensity = %s", name, number(row$intensity)))
    if (with_points) {
      out <- c(out, sprintf("%s.n_points = %d", name, as.integer(row$n_points)))
    }
  }
  out
}

main <- function() {
  args <- commandArgs(trailingOnly = TRUE)
  directory <- if (length(args) > 0) args[1] else "core/tests/fixtures/api"
  fixture <- file.path(directory, "api.ion")

  file <- parse_ion(fixture)
  transitions <- parse_ion(file.path(dirname(directory), "srm.ion"))
  out <- character(0)
  out <- c(out, sprintf("fnv1a.self_check = %s", fnv1a(SELF_CHECK_VALUES)))

  scans <- get_scans(file, rt = c(FROM_RT, TO_RT))
  out <- c(out, sprintf("parse_ion.scans = %d", nrow(scans)))
  last_index <- scans$index[nrow(scans)]
  out <- c(out, sprintf("get_scans.first.index = %.0f", scans$index[1]),
           sprintf("get_scans.last.index = %.0f", last_index))
  rows <- get_scan_metadata(file, last_index)
  out <- c(out, sprintf("get_scan_metadata.count = %d", nrow(rows)))
  for (position in seq_len(nrow(rows))) {
    fields <- unlist(rows[position, ], use.names = FALSE)
    fields[is.na(fields)] <- "NA"
    out <- c(out, sprintf("get_scan_metadata.%d = %s", position - 1, paste(fields, collapse = " | ")))
  }
  scan_rows <- get_scan_metadata(file, last_index, get_scan_metadata_options(tags = "scan"))
  out <- c(out, sprintf("get_scan_metadata.scan.count = %d", nrow(scan_rows)))

  for (channel in CHANNELS) {
    out <- report_channel(file, transitions, channel, out)
  }

  found <- find_features(file, find_features_options(
    eic = window_options(), grid = grid(), find_peaks = feature_peak_options(), threads = THREADS))
  out <- feature_lines("find_features", found, out, TRUE)

  consensus <- match_features(directory, match_features_options(
    eic = window_options(), grid = grid(), find_peaks = feature_peak_options(), threads = THREADS))
  out <- feature_lines("match_features", consensus, out, FALSE)
  out <- feature_lines("deisotope_features", deisotope_features(consensus), out, FALSE)

  warp <- load_warp(load_model(file.path(directory, "..", "shift_model.onnx")))
  ids <- unlist(lapply(CHANNELS, function(channel)
    vapply(channel$targets, function(target) paste0(channel$name, ".", target[1]), character(1))))
  targets <- data.frame(
    id = ids,
    mz = unlist(lapply(CHANNELS, function(channel) rep(channel$mass, length(channel$targets)))),
    rt = unlist(lapply(CHANNELS, function(channel)
      vapply(channel$targets, function(target) as.numeric(target[2]), numeric(1))))
  )
  predicted <- predict_rts(warp, file, targets)
  out <- c(out, sprintf("predict_rts.count = %d", nrow(predicted)))
  for (index in seq_len(nrow(predicted))) {
    out <- c(out,
      sprintf("predict_rts.%s.rt = %s", ids[index], number(predicted$rt[index])),
      sprintf("predict_rts.%s.uncertainty = %s", ids[index], number(predicted$uncertainty[index])))
  }

  cat(paste(out, collapse = "\n"), "\n", sep = "")
}

main()
