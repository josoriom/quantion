\name{find_peaks}
\alias{find_peaks}
\title{Find every peak in a trace}
\description{
Find every peak in one trace.
}
\usage{
find_peaks(x, y, options = find_peaks_options())
}
\arguments{
  \item{x}{Retention times, ascending. At least 3 points. Type: \code{numeric}.}
  \item{y}{Intensities, same length as x. Type: \code{numeric}.}
  \item{options}{Settings from \code{\link{find_peaks_options}()}: \code{min_integral = NA}, \code{min_intensity = 500}, \code{min_peak_width_points = 3L}, \code{noise = NA}, \code{auto_noise = TRUE}, \code{auto_baseline = TRUE}, \code{lambda = 20}, \code{max_iterations = 4L}, \code{allow_overlap = FALSE}, \code{min_snr = 2}, \code{min_r2 = 0}, \code{shape = "EMG"}, \code{kernel_size = 0L}.
    \href{https://phenological.github.io/quantion/?language=r#FindPeaksOptions}{FindPeaksOptions documentation}
  }
}
\value{
Every peak that passes the filters. Type: \code{data.frame of Peak}. Columns: \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#find_peaks}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  eic <- calculate_eic(sample, 260.103, options = calculate_eic_options(from = 2, to = 3))
  peaks <- find_peaks(eic$x, eic$y, options = find_peaks_options(min_snr = 3))
  print(peaks)
}
}
}
\seealso{
\code{\link{find_peaks_options}()}, \code{\link{get_peak}()}
}
