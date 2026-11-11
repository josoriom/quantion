srm_file <- {
  root <- find_sample_root()
  if (nzchar(root)) file.path(root, "core", "tests", "fixtures", "srm.ion") else ""
}

get_target <- function(precursor, product) {
  data.frame(id = "alanine", precursor = precursor, product = product, rt = 2.385, range = 0.1)
}

test_that("a target finds its transition", {
  skip_if(!nzchar(srm_file), "no srm.ion fixture next to the package")
  row <- find_transitions(parse_ion(srm_file), get_target(89.04768, 89.04768))
  expect_gt(row$intensity, 0)
  expect_equal(row$rt, 2.385, tolerance = 0.05)
})

test_that("a wrong product finds nothing", {
  skip_if(!nzchar(srm_file), "no srm.ion fixture next to the package")
  row <- find_transitions(parse_ion(srm_file), get_target(89.04768, 90.0555))
  expect_equal(row$intensity, 0)
})

test_that("a missing product column is refused", {
  skip_if(!nzchar(srm_file), "no srm.ion fixture next to the package")
  targets <- data.frame(id = "alanine", precursor = 89.04768, rt = 2.385, range = 0.1)
  error <- tryCatch(find_transitions(parse_ion(srm_file), targets), quantion_error = function(error) error)
  expect_identical(error$kind, "missing_argument")
  expect_identical(error$argument, "targets$product")
})
