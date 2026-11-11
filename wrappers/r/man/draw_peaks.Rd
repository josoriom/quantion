\name{draw_peaks}
\alias{draw_peaks}
\title{Simulate a chromatogram}
\description{
Simulate a chromatogram from one or more peak shapes. Every peak is drawn into the same chromatogram; where they overlap, intensities add up. No real data is needed.
}
\usage{
draw_peaks(params, options = draw_peaks_options())
}
\arguments{
  \item{params}{The peaks to draw, one PeakParameters per peak. Type: \code{list of PeakParameters}. Each PeakParameters has \code{height}, \code{center}, \code{fwhm}, \code{tail}, \code{r2}.
    Example: \code{peak <- list(height = 9800, center = 2.38, fwhm = 0.021, tail = 0.006)}
    \href{https://phenological.github.io/quantion/?language=r#PeakParameters}{PeakParameters documentation}
  }
  \item{options}{Settings from \code{\link{draw_peaks_options}()}: \code{from = 0}, \code{to = 0}, \code{points = 1000L}.
    \href{https://phenological.github.io/quantion/?language=r#DrawPeaksOptions}{DrawPeaksOptions documentation}
  }
}
\value{
The simulated chromatogram: x = retention times, y = summed intensities. Type: \code{DataXY list}. Fields: \code{x}, \code{y}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#draw_peaks}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  peaks <- list(
    list(height = 9800, center = 2.38, fwhm = 0.021, tail = 0.006),
    list(height = 4200, center = 2.45, fwhm = 0.018, tail = 0),
    list(height = 1500, center = 2.60, fwhm = 0.030, tail = 0.010)
  )
  chromatogram <- draw_peaks(peaks, options = draw_peaks_options(from = 2, to = 3, points = 2000L))
  fitted <- draw_peaks(peaks)
  print(head(data.frame(rt = chromatogram$x, intensity = chromatogram$y)))
}
}
}
\seealso{
\code{\link{draw_peaks_options}()}, \code{\link{fit_peak}()}
}
