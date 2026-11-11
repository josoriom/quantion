\name{find_targets_options}
\alias{find_targets_options}
\title{Settings for find_targets}
\description{
Settings for find_targets.
}
\usage{
find_targets_options(eic = calculate_eic_options(), find_peaks = find_peaks_options(),
                     threads = 1L)
}
\arguments{
  \item{eic}{EIC retention-time range (from, to), tolerances and time unit. See \code{\link{calculate_eic_options}()}.}
  \item{find_peaks}{Settings for peak picking. See \code{\link{find_peaks_options}()}.}
  \item{threads}{Number of threads.}
}
\value{
An options object for \code{\link{find_targets}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#FindTargetsOptions}
}
\examples{
find_targets_options(eic = calculate_eic_options(from = 0.5, to = 5), threads = 4L)
}
\seealso{
\code{\link{find_targets}()}
}
