\name{grouping_options}
\alias{grouping_options}
\title{How features are matched across samples}
\description{
How match_features matches features across samples.
}
\usage{
grouping_options(ppm_tolerance = 5, mz_tolerance = 0.0025, rt_tolerance = 0.05,
                 min_frequency = 0)
}
\arguments{
  \item{ppm_tolerance}{Same feature when m/z is within this ppm....}
  \item{mz_tolerance}{...or within this absolute m/z (Da).}
  \item{rt_tolerance}{...and rt is within this many minutes.}
  \item{min_frequency}{Keep a feature only if found in at least this fraction of samples (0-1).}
}
\value{
An options object for \code{\link{match_features_options}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#GroupingOptions}
}
\examples{
grouping_options(min_frequency = 0.5)
}
\seealso{
\code{\link{match_features_options}()}
}
