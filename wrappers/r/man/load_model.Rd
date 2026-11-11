\name{load_model}
\alias{load_model}
\title{Load an ONNX model}
\description{
Load an ONNX model from a path, a URL or bytes, and read the keys and values stored in it.
}
\usage{
load_model(source)
}
\arguments{
  \item{source}{Where the model is: a path, an http(s) URL, or raw bytes. Type: \code{character | raw}.}
}
\value{
The model, with get_keys(), get_values() and get_value(key). Type: \code{Model}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#load_model}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  model <- load_model(paste0(fixtures, "/shift_model.onnx"))
  points <- get_value(model, "points")
  print(points)

  dispose(model)
}
}
}
\seealso{
\code{\link{load_warp}()}, \code{\link{get_keys}()}
}
