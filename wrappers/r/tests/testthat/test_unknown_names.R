sample_file <- sample_ion_path()

get_message <- function(call) {
  error <- tryCatch(call(), quantion_error = function(error) error)
  expect_s3_class(error, "quantion_error")
  expect_equal(error$kind, "unknown_option")
  conditionMessage(error)
}

test_that("a target column that is not an option is refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  file <- parse_ion(sample_file)
  targets <- data.frame(mz = 89.04768, rt = 2.385, rnage = 0.1)
  expect_equal(get_message(function() find_targets(file, targets)),
               "find_targets: targets$rnage is not an option; the options are mz, rt, id, range")
})

test_that("a transition column that is not an option is refused", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  file <- parse_ion(sample_file)
  transitions <- data.frame(precursor = 89.04768, product = 89.04768, rt = 2.385, range = 0.1, prodcut = 1)
  expect_equal(get_message(function() find_transitions(file, transitions)),
               "find_transitions: targets$prodcut is not an option; the options are precursor, product, rt, range, id")
})

test_that("a peak name that is not an option is refused", {
  peaks <- list(list(height = 1, center = 2, fwhm = 0.1, tail = 0, hieght = 5))
  expect_equal(get_message(function() draw_peaks(peaks)),
               "draw_peaks: params[[1]]$hieght is not an option; the options are height, center, fwhm, tail, r2")
})

test_that("a peak without tail is refused", {
  peaks <- list(list(height = 1, center = 2, fwhm = 0.1))
  error <- tryCatch(draw_peaks(peaks), quantion_error = function(error) error)
  expect_equal(error$kind, "missing_argument")
  expect_equal(conditionMessage(error), "draw_peaks: params[[1]]$tail is required")
})
