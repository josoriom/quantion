sample_file <- sample_ion_path()

get_targets <- function() {
  data.frame(id = c("a", "b"), rt = c(2.385, 2.18), mz = c(89.04768, 89.04768), range = c(0.1, 0.1))
}

test_that("targets give one peak per target", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  options <- find_targets_options(eic = calculate_eic_options(from = 0, to = 5), threads = 2L)
  peaks <- find_targets(parse_ion(sample_file), get_targets(), options)
  expect_equal(nrow(peaks), 2)
  expect_named(peaks, c("rt", "from", "to", "intensity", "integral", "n_points"))
  expect_true(all(peaks$intensity > 0))
})

test_that("a target without range searches its whole EIC", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  targets <- data.frame(mz = 89.04768, rt = 2.385)
  peaks <- find_targets(parse_ion(sample_file), targets)
  expect_gt(peaks$intensity, 0)
})

test_that("a missing mz column is a missing_argument error", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  targets <- data.frame(id = "a", rt = 2.385)
  error <- tryCatch(find_targets(parse_ion(sample_file), targets), quantion_error = function(error) error)
  expect_identical(error$kind, "missing_argument")
  expect_identical(error$argument, "targets$mz")
})

test_that("seconds give the same peaks as minutes", {
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")
  in_minutes <- find_targets(parse_ion(sample_file), get_targets(),
                             find_targets_options(eic = calculate_eic_options(from = 0, to = 5)))
  in_seconds <- get_targets()
  in_seconds$rt <- in_seconds$rt * 60
  in_seconds$range <- in_seconds$range * 60
  options <- find_targets_options(eic = calculate_eic_options(from = 0, to = 300, time_unit = "SECONDS"))
  expect_equal(find_targets(parse_ion(sample_file), in_seconds, options), in_minutes)
})
