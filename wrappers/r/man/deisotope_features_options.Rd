\name{deisotope_features_options}
\alias{deisotope_features_options}
\title{Settings for deisotope_features}
\description{
How deisotope_features links isotope peaks.
}
\usage{
deisotope_features_options(rt_tolerance = 0.02, mz_tolerance = 0.005)
}
\arguments{
  \item{rt_tolerance}{Same compound when rt is within this many minutes of M.}
  \item{mz_tolerance}{Isotope peak when m/z is within this many Da of the expected position.}
}
\value{
A \code{deisotope_features_options} object for \code{\link{deisotope_features}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#DeisotopeFeaturesOptions}
}
