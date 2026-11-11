suppressMessages(library(quantion))

MASS <- 90.05550
FROM_RT <- 0.0
TO_RT <- 5.0
EIC_PPM <- 20.0
EIC_MZ <- 0.005

same_bits <- function(left, right) {
  if (length(left) != length(right)) return(FALSE)
  all(mapply(function(a, b) {
    identical(writeBin(a, raw(), size = 8), writeBin(b, raw(), size = 8))
  }, left, right))
}

.origin <- function(url) sub("^(https?://[^/]+).*$", "\\1", url)

.stats <- function(url, path) {
  text <- paste(readLines(paste0(.origin(url), path), warn = FALSE), collapse = "")
  numbers <- as.numeric(regmatches(text, gregexpr("[0-9]+", text))[[1]])
  list(bytes_sent = numbers[1], requests = numbers[2])
}

main <- function() {
  url <- Sys.getenv("QUANTION_REMOTE_URL", "")
  if (!nzchar(url)) {
    message("QUANTION_REMOTE_URL is not set")
    return(1L)
  }
  fixture <- "core/tests/fixtures/api/api.ion"

  invisible(.stats(url, "/reset"))
  remote <- parse_ion(url)
  opening <- .stats(url, "/stats")

  invisible(.stats(url, "/reset"))
  local <- parse_ion(fixture)

  window <- calculate_eic_options(from = FROM_RT, to = TO_RT, ppm_tolerance = EIC_PPM, mz_tolerance = EIC_MZ)
  eic_remote <- calculate_eic(remote, MASS, window)
  eic_local <- calculate_eic(local, MASS, window)
  one_query <- .stats(url, "/stats")

  total <- file.info(fixture)$size
  same <- same_bits(eic_remote$x, eic_local$x) && same_bits(eic_remote$y, eic_local$y)

  cat(sprintf("file_bytes = %.0f\n", total))
  cat(sprintf("opening_bytes = %.0f\n", opening$bytes_sent))
  cat(sprintf("opening_requests = %.0f\n", opening$requests))
  cat(sprintf("query_bytes = %.0f\n", one_query$bytes_sent))
  cat(sprintf("query_requests = %.0f\n", one_query$requests))
  cat(sprintf("matches_local = %s\n", if (same) "yes" else "no"))

  warp <- load_warp(load_model("core/tests/fixtures/shift_model.onnx"))
  target <- data.frame(id = "a", rt = 2.385, mz = 89.04768, range = 0.1)
  calls <- list(
    calculate_eic = function(file) calculate_eic(file, MASS, calculate_eic_options(from = FROM_RT, to = TO_RT)),
    get_scans = function(file) get_scans(file, rt = c(2.3, 2.5)),
    get_ion_image = function(file) get_ion_image(file, MASS),
    find_targets = function(file) find_targets(file, target, find_targets_options(eic = calculate_eic_options(from = FROM_RT, to = TO_RT))),
    find_transitions = function(file) find_transitions(file, data.frame(id = "a", precursor = 89.04768, product = 89.04768, rt = 2.385, range = 0.1)),
    find_features = function(file) find_features(file, find_features_options(eic = calculate_eic_options(from = FROM_RT, to = TO_RT))),
    write_ion = function(file) write_ion(file),
    write_mzml = function(file) write_mzml(file),
    predict_rts = function(file) predict_rts(warp, file, data.frame(mz = 250.1, rt = 1.2))
  )
  for (name in names(calls)) {
    cat(sprintf("fresh_%s = %s\n", name, compare(calls[[name]], url, fixture)))
  }
  cat(sprintf("upload = %s\n", check_upload(url, local, window, eic_local)))
  0L
}

check_upload <- function(url, local, window, eic_local) {
  target <- paste0(sub("/[^/]*$", "", url), "/uploaded.ion")
  tryCatch({
    write_ion(local, write_ion_options(out = target, headers = list(header("x-check", "upload"))))
    uploaded <- calculate_eic(parse_ion(target), MASS, window)
    if (same_bits(uploaded$x, eic_local$x) && same_bits(uploaded$y, eic_local$y)) "same" else "different"
  }, quantion_error = function(error) paste("error:", conditionMessage(error)))
}

compare <- function(call, url, fixture) {
  remote <- tryCatch(call(parse_ion(url)), error = function(e) e)
  if (inherits(remote, "error")) {
    if (inherits(remote, "quantion_error") && identical(remote$kind, "remote_file")) return("refused")
    text <- strsplit(conditionMessage(remote), "\n")[[1]][1]
    return(paste("error", substr(text, 1, 80)))
  }
  local <- call(parse_ion(fixture))
  if (identical(remote, local)) "same" else "differs"
}

quit(status = main())
