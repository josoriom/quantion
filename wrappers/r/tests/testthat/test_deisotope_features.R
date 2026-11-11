isotope_diff <- 1.003354835

make_features <- function(mz, intensity) {
  data.frame(mz = mz, rt = 5, from = 4.9, to = 5.1, intensity = intensity, integral = intensity, n_points = 9L)
}

test_that("deisotope_features removes the isotope and sorts by mz", {
  found <- deisotope_features(make_features(c(300, 200 + isotope_diff, 200), c(500, 100, 1000)))
  expect_equal(found$mz, c(200, 300))
  expect_equal(found$n_points, c(9L, 9L))
  expect_false("frequency" %in% names(found))
})

test_that("deisotope_features keeps frequency", {
  features <- make_features(200, 1000)
  features$frequency <- 0.5
  expect_equal(deisotope_features(features)$frequency, 0.5)
})

test_that("deisotope_features uses the options", {
  features <- make_features(c(200, 200 + isotope_diff + 0.003), c(1000, 100))
  expect_equal(nrow(deisotope_features(features)), 1)
  expect_equal(nrow(deisotope_features(features, deisotope_features_options(mz_tolerance = 0.001))), 2)
})

test_that("deisotope_features accepts an empty table", {
  expect_equal(nrow(deisotope_features(make_features(200, 1000)[0, ])), 0)
})

test_that("deisotope_features rejects unknown names and invalid values", {
  expect_error(deisotope_features(make_features(200, 1000), deisotope_features_options(rt_tolerance = -1)))
  expect_error(deisotope_features(cbind(make_features(200, 1000), extra = 1)))
  expect_error(deisotope_features(data.frame(mz = 200)))
  expect_error(deisotope_features("features"))
})
