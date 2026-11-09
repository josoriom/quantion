\name{find_noise_level}
\alias{find_noise_level}
\title{Estimate the noise level}
\description{
Estimate the noise level of a signal.
}
\usage{
find_noise_level(y)
}
\arguments{
  \item{y}{Intensities. Type: \code{numeric}.}
}
\value{
The noise intensity and the window width it came from. Type: \code{NoiseLevel list}. Fields: \code{width}, \code{intensity}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#find_noise_level}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  eic <- calculate_eic(sample, 260.103, options = calculate_eic_options(from = 2, to = 3))
  noise <- find_noise_level(eic$y)$intensity
  print(noise)
}
}
}
\seealso{
\code{\link{find_peaks}()}
}
