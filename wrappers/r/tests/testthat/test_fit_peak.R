sample_file <- sample_ion_path()

get_eic <- function() {
  calculate_eic(parse_ion(sample_file), 89.04768, calculate_eic_options(from = 0, to = 5))
}

test_that("a fit has no shape field and an EMG has a tail", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  eic <- get_eic()
  fit <- fit_peak(eic$x, eic$y, rt = 2.385)
  expect_named(fit, c("height", "center", "fwhm", "tail", "r2"))
  expect_gt(fit$tail, 0)
})

test_that("a Gaussian fit has no tail", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  eic <- get_eic()
  fit <- fit_peak(eic$x, eic$y, rt = 2.385, options = fit_peak_options(shape = "GAUSSIAN"))
  expect_identical(fit$tail, 0)
})

test_that("a drawn Gaussian is fitted back", {
  skip_if_not_installed("quantion")
  peak <- list(height = 1000, center = 2, fwhm = 0.05, tail = 0)
  drawn <- draw_peaks(list(peak), draw_peaks_options(from = 1, to = 3, points = 401L))
  fit <- fit_peak(drawn$x, drawn$y, rt = 2, options = fit_peak_options(shape = "GAUSSIAN"))
  expect_equal(fit$center, 2, tolerance = 0.01)
})

test_that("peaks add up where they overlap", {
  skip_if_not_installed("quantion")
  peaks <- list(list(height = 100, center = 2, fwhm = 0.05, tail = 0), list(height = 50, center = 2, fwhm = 0.05, tail = 0))
  drawn <- draw_peaks(peaks, draw_peaks_options(from = 1.5, to = 2.5, points = 101L))
  expect_equal(max(drawn$y), 150)
})

test_that("the drawing range fits the peaks when not given", {
  skip_if_not_installed("quantion")
  peaks <- list(list(height = 100, center = 2, fwhm = 0.05, tail = 0), list(height = 100, center = 3, fwhm = 0.05, tail = 0))
  drawn <- draw_peaks(peaks)
  expect_length(drawn$x, 1000)
  expect_true(drawn$x[1] < 2 && drawn$x[1000] > 3)
})

test_that("without range get_peak searches the whole trace", {
  skip_if_not_installed("quantion")
  trace <- draw_peaks(list(list(height = 10000, center = 8, fwhm = 0.05, tail = 0)),
                      draw_peaks_options(from = 0, to = 10, points = 2001L))
  near <- get_peak(trace$x, trace$y, rt = 2, options = get_peak_options(range = 0.5))
  anywhere <- get_peak(trace$x, trace$y, rt = 2)
  expect_identical(near$intensity, 0)
  expect_equal(anywhere$rt, 8, tolerance = 0.05)
})
