sample_file <- sample_ion_path()

get_error <- function(work) tryCatch(work, quantion_error = function(error) error)

test_that("a Rust error keeps its kind, code, function, argument and message", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  error <- get_error(calculate_eic(parse_ion(sample_file), -1))
  expect_s3_class(error, "quantion_error")
  expect_identical(error$kind, "invalid_argument")
  expect_identical(error$code, 1L)
  expect_identical(error$`function`, "calculate_eic")
  expect_identical(error$argument, "mz")
  expect_identical(conditionMessage(error), "calculate_eic: mz must be greater than 0, got -1")
})

test_that("a missing file is a file_not_found error", {
  skip_if_not_installed("quantion")
  error <- get_error(parse_ion(file.path(tempdir(), "missing.ion")))
  expect_identical(error$kind, "file_not_found")
  expect_identical(error$code, 5L)
})

test_that("options must come from their helper", {
  skip_if_not_installed("quantion")
  error <- get_error(find_peaks(1:5, 1:5, list(min_snr = 3)))
  expect_identical(error$kind, "invalid_argument")
  expect_identical(conditionMessage(error), "find_peaks: options must be built with find_peaks_options()")
})

test_that("a group must come from its helper too", {
  skip_if_not_installed("quantion")
  error <- get_error(find_targets_options(eic = list(from = 1)))
  expect_identical(error$argument, "eic")
})

test_that("enums are uppercase", {
  skip_if_not_installed("quantion")
  expect_identical(calculate_eic_options(time_unit = "SECONDS")$time_unit, "SECONDS")
  expect_identical(fit_peak_options(shape = "GAUSSIAN")$shape, "GAUSSIAN")
  error <- get_error(calculate_eic_options(time_unit = "minutes"))
  expect_identical(error$argument, "time_unit")
})

test_that("ion bytes open again and give the same EIC", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  sample <- parse_ion(sample_file)
  again <- parse_ion(write_ion(sample))
  expect_identical(calculate_eic(again, 89.04768)$y, calculate_eic(sample, 89.04768)$y)
})

test_that("an existing output needs overwrite", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  sample <- parse_ion(sample_file)
  path <- tempfile(fileext = ".ion")
  on.exit(unlink(path), add = TRUE)
  expect_null(write_ion(sample, write_ion_options(out = path)))
  error <- get_error(write_ion(sample, write_ion_options(out = path)))
  expect_identical(error$kind, "file_exists")
  expect_null(write_ion(sample, write_ion_options(out = path, overwrite = TRUE)))
})

test_that("headers need a URL", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  options <- write_ion_options(headers = list(header("Authorization", "Bearer token")))
  error <- get_error(write_ion(parse_ion(sample_file), options))
  expect_identical(error$argument, "headers")
})

test_that("mzML bytes open again, but cannot be imaged", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  mzml <- parse_mzml(write_mzml(parse_ion(sample_file)))
  expect_equal(nrow(get_scans(mzml, rt = 2.385)), 1)
  error <- get_error(get_ion_image(mzml, 89.04768))
  expect_identical(error$argument, "file")
})

test_that("an ion image has its shape, values and counts", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  image <- get_ion_image(parse_ion(sample_file), 89.04768)
  expect_named(image, c("width", "height", "min_x", "min_y", "min_z", "max_z", "data", "counts"))
  expect_length(image$data, image$width * image$height)
})

test_that("a disposed sample cannot be used", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  sample <- parse_ion(sample_file)
  dispose(sample)
  error <- get_error(calculate_eic(sample, 89.04768))
  expect_identical(error$argument, "file")
})
