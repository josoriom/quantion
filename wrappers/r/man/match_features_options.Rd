\name{match_features_options}
\alias{match_features_options}
\title{Settings for match_features}
\description{
Settings for match_features: the find_features settings plus grouping.
}
\usage{
match_features_options(eic = calculate_eic_options(), grid = grid_options(),
                       grouping = grouping_options(), find_peaks = find_peaks_options(),
                       threads = 1L)
}
\arguments{
  \item{eic}{EIC retention-time range (from, to), tolerances and time unit. See \code{\link{calculate_eic_options}()}.}
  \item{grid}{The m/z grid. See \code{\link{grid_options}()}.}
  \item{grouping}{How features are matched across samples. See \code{\link{grouping_options}()}.}
  \item{find_peaks}{Settings for peak picking. See \code{\link{find_peaks_options}()}.}
  \item{threads}{Number of threads.}
}
\value{
An options object for \code{\link{match_features}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#MatchFeaturesOptions}
}
\examples{
match_features_options(grouping = grouping_options(min_frequency = 0.5), threads = 8L)
}
\seealso{
\code{\link{match_features}()}
}
