\name{predict_rts}
\alias{predict_rts}
\title{Predict retention times}
\description{
Predict where each target elutes in one sample.
}
\usage{
predict_rts(warp, file, targets)
}
\arguments{
  \item{warp}{The warp model. Type: \code{WarpModel}.}
  \item{file}{The sample to read, from parse_ion or parse_mzml. Type: \code{SampleFile}.}
  \item{targets}{Targets with mz and reference rt. id and range are optional and not used. Type: \code{data.frame of Target}. Each Target has \code{id}, \code{mz}, \code{rt}, \code{range}.
    Example: \code{targets <- data.frame(id = "compound_1", mz = 260.103, rt = 2.383, range = 0.05)}
    \href{https://phenological.github.io/quantion/?language=r#Target}{Target documentation}
  }
}
\value{
One PredictedRt per target, in the same order. Type: \code{data.frame of PredictedRt}. Columns: \code{rt}, \code{uncertainty}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#predict_rts}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  model <- load_model(paste0(fixtures, "/shift_model.onnx"))
  warp <- load_warp(model)
  sample <- parse_ion(url)
  predicted <- predict_rts(warp, sample, data.frame(id = "compound_1", mz = 260.103, rt = 2.383))
  print(predicted)
}
}
}
\seealso{
\code{\link{load_warp}()}, \code{\link{calibrate}()}
}
