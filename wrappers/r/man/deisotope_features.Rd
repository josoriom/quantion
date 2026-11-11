\name{deisotope_features}
\alias{deisotope_features}
\title{Remove isotope peaks from a feature list}
\description{
Remove 13C isotope peaks from a feature list, keeping the monoisotopic peak (M) of each compound. A feature is an isotope when it sits 1.00335/charge above the previous peak (charge = 1 or 2), has the same rt as M, and its intensity is not too high: M+1 at most M x mz x charge / 12 x 0.0108157.
}
\usage{
deisotope_features(features, options = deisotope_features_options())
}
\arguments{
  \item{features}{The features to clean, from find_features or match_features. Type: \code{data.frame of Feature}.}
  \item{options}{Settings from \code{\link{deisotope_features_options}()}: \code{rt_tolerance = 0.02}, \code{mz_tolerance = 0.005}.
    \href{https://phenological.github.io/quantion/?language=r#DeisotopeFeaturesOptions}{DeisotopeFeaturesOptions documentation}
  }
}
\value{
The features that are not isotopes, sorted by m/z. Type: \code{data.frame of Feature}. Columns: \code{mz}, \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}, \code{frequency}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#deisotope_features}
}
