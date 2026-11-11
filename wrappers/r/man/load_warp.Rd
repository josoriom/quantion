\name{load_warp}
\alias{load_warp}
\title{Make a retention-time warp model}
\description{
Make a retention-time warp model from a loaded model.
}
\usage{
load_warp(model)
}
\arguments{
  \item{model}{A model from load_model that holds a warp. Type: \code{Model}.}
}
\value{
The warp model. Type: \code{WarpModel}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#load_warp}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  model <- load_model(paste0(fixtures, "/shift_model.onnx"))
  warp <- load_warp(model)
  print(warp)

  dispose(warp)
  dispose(model)
}
}
}
\seealso{
\code{\link{predict_rts}()}, \code{\link{calibrate}()}
}
