test_that("the replaced functions are gone", {
  skip_if_not_installed("quantion")
  exports <- getNamespaceExports("quantion")
  for (name in c("mzml_to_ion", "mzml_to_ion_file", "ion_to_mzml", "draw_peak")) {
    expect_false(name %in% exports, info = name)
  }
})
