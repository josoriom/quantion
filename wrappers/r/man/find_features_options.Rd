\name{find_features_options}
\alias{find_features_options}
\title{Settings for find_features}
\description{
Settings for find_features.
}
\usage{
find_features_options(eic = calculate_eic_options(), grid = grid_options(),
                      find_peaks = find_peaks_options(), threads = 1L)
}
\arguments{
  \item{eic}{EIC retention-time range (from, to), tolerances and time unit. See \code{\link{calculate_eic_options}()}.}
  \item{grid}{The m/z grid. See \code{\link{grid_options}()}.}
  \item{find_peaks}{Settings for peak picking. See \code{\link{find_peaks_options}()}.}
  \item{threads}{Number of threads.}
}
\value{
An options object for \code{\link{find_features}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#FindFeaturesOptions}
}
\examples{
find_features_options(grid = grid_options(start = 100, end = 600), threads = 4L)
}
\seealso{
\code{\link{find_features}()}
}
