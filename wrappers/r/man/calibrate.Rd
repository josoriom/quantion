\name{calibrate}
\alias{calibrate}
\title{Calibrate reference retention times}
\description{
Turn the retention times seen in anchor samples (LTRs, QCs) into targets for predict_rts. Each id gets the median rt over the anchors.
}
\usage{
calibrate(warp, anchor_samples, target_lists)
}
\arguments{
  \item{warp}{The warp model. Type: \code{WarpModel}.}
  \item{anchor_samples}{Anchor samples. Type: \code{list of SampleFile}.}
  \item{target_lists}{For each anchor sample, the targets seen in it with their observed rt. Type: \code{list of data.frame}.}
}
\value{
One Target per id with the median rt. Type: \code{data.frame of Target}. Columns: \code{id}, \code{mz}, \code{rt}, \code{range}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#calibrate}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  model <- load_model(paste0(fixtures, "/shift_model.onnx"))
  warp <- load_warp(model)
  anchor_1 <- parse_ion(url)
  anchor_2 <- parse_ion(url)
  anchors <- list(anchor_1, anchor_2)
  seen <- list(
    data.frame(id = "compound_1", mz = 260.103, rt = 2.381),
    data.frame(id = "compound_1", mz = 260.103, rt = 2.392)
  )
  targets <- calibrate(warp, anchors, seen)
  print(targets)
}
}
}
\seealso{
\code{\link{predict_rts}()}
}
