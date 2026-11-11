sample_file <- sample_ion_path()

test_that("a remote sample is read over range requests", {
  skip_if_not_installed("quantion")
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")

  with_served_file(sample_file, TRUE, function(url) {
    handle <- quantion::parse_ion(url)
    expect_identical(typeof(handle), "externalptr")

    source <- attr(handle, "quantion_source")
    expect_false(is.null(source))
    expect_gt(length(source$held), 1)
  })
})

test_that("the total size of a remote sample comes from the plan, not from a guess", {
  skip_if_not_installed("quantion")
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")

  with_served_file(sample_file, TRUE, function(url) {
    handle <- quantion::parse_ion(url)
    source <- attr(handle, "quantion_source")

    expect_equal(source$total, file.info(sample_file)$size)
    expect_gt(source$total, 1024)
  })
})

test_that("every block a remote sample holds sits inside the file", {
  skip_if_not_installed("quantion")
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")

  with_served_file(sample_file, TRUE, function(url) {
    handle <- quantion::parse_ion(url)
    source <- attr(handle, "quantion_source")
    blocks <- do.call(rbind, source$held)

    expect_true(all(blocks[, 1] >= 0))
    expect_true(all(blocks[, 2] > 0))
    expect_true(all(blocks[, 1] + blocks[, 2] <= source$total))
  })
})

test_that("an extracted ion chromatogram needs no bytes the open did not bring in", {
  skip_if_not_installed("quantion")
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")

  with_served_file(sample_file, TRUE, function(url) {
    handle <- quantion::parse_ion(url)
    source <- attr(handle, "quantion_source")
    held_before <- length(source$held)

    eic <- quantion::calculate_eic(handle, 89.04768, quantion::calculate_eic_options(from = 0.5, to = 1.5))

    expect_named(eic, c("x", "y"))
    expect_gt(length(eic$x), 0)
    expect_length(source$held, held_before)
  })
})

test_that("a server that ignores range requests is reported", {
  skip_if_not_installed("quantion")
  skip_if_not_installed("callr")
  skip_if(!nzchar(sample_file), "no api.ion sample next to the package")

  with_served_file(sample_file, FALSE, function(url) {
    expect_error(quantion::parse_ion(url), "does not support range requests")
  })
})
