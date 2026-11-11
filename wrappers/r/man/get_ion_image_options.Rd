\name{get_ion_image_options}
\alias{get_ion_image_options}
\title{Settings for get_ion_image}
\description{
Settings for get_ion_image.
}
\usage{
get_ion_image_options(tolerance = 0.125, level = 1L, on_progress = NULL)
}
\arguments{
  \item{tolerance}{Absolute m/z tolerance (Da).}
  \item{level}{MS level.}
  \item{on_progress}{Progress callback: fetched bytes, total bytes, bytes held.}
}
\value{
An options object for \code{\link{get_ion_image}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#GetIonImageOptions}
}
\examples{
get_ion_image_options(tolerance = 0.05)
}
\seealso{
\code{\link{get_ion_image}()}
}
