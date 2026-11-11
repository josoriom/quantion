\name{annotate_features_options}
\alias{annotate_features_options}
\title{Settings for annotate_features}
\description{
Settings for annotate_features.
}
\usage{
annotate_features_options(library = .DEFAULT_LIBRARY, model = .DEFAULT_MODEL,
                          matches = 3L, ppm_tolerance = 20, mz_tolerance = 0.005,
                          rt_range = 0.3, min_relative_intensity = 0.02)
}
\arguments{
  \item{library}{Spectral library: an Ionic (.ion) file path or URL. Each spectrum has a precursor m/z, a vector array and the params named by the model.}
  \item{model}{Spectrum model: an ONNX file path or URL.}
  \item{matches}{How many matches to keep per file, best first.}
  \item{ppm_tolerance}{Precursor tolerance in ppm. The larger of the two tolerances is used.}
  \item{mz_tolerance}{Precursor tolerance in Da.}
  \item{rt_range}{Largest distance in minutes from the peak apex to the MS2 scan.}
  \item{min_relative_intensity}{MS2 peaks below this share of the base peak are dropped (0-1).}
}
\value{
A \code{annotate_features_options} object for \code{\link{annotate_features}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#AnnotateFeaturesOptions}
}
