\name{get_keys}
\alias{get_keys}
\alias{get_values}
\alias{get_value}
\title{Read the keys and values stored in a model}
\description{
ONNX models store text under keys. These functions read them from a model or
from a warp model.
}
\usage{
get_keys(model)
get_values(model)
get_value(model, key)
}
\arguments{
  \item{model}{A model from \code{\link{load_model}()} or a warp model from \code{\link{load_warp}()}.}
  \item{key}{A key, like \code{"points"}.}
}
\value{
\code{get_keys()} gives the keys in file order. \code{get_values()} gives the
values in the same order. \code{get_value()} gives the text stored under
\code{key}, or \code{NULL} if it is missing.
}
\examples{
model <- load_model(system.file("extdata", "model.onnx", package = "quantion"))
get_keys(model)
get_value(model, "points")
}
\seealso{
\code{\link{load_model}()}, \code{\link{load_warp}()}
}
