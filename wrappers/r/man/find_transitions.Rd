\name{find_transitions}
\alias{find_transitions}
\title{Find the peak of each SRM transition}
\description{
For each SRM transition, take the stored chromatogram whose precursor (Q1) and product (Q3) are both within 0.005 m/z, and find its peak near rt +/- range.
}
\usage{
find_transitions(file, targets, options = find_transitions_options())
}
\arguments{
  \item{file}{The sample to read, from parse_ion or parse_mzml. Type: \code{SampleFile}.}
  \item{targets}{The transitions. Type: \code{data.frame of Transition}. Each Transition has \code{id}, \code{precursor}, \code{product}, \code{rt}, \code{range}.
    Example: \code{                          rt = 2.385, range = 0.1)}
    \href{https://phenological.github.io/quantion/?language=r#Transition}{Transition documentation}
  }
  \item{options}{Settings from \code{\link{find_transitions_options}()}: \code{find_peaks}, \code{threads = 1L}.
    \href{https://phenological.github.io/quantion/?language=r#FindTransitionsOptions}{FindTransitionsOptions documentation}
  }
}
\value{
One Peak per transition, in the same order. No match gives a Peak with every field 0. Type: \code{data.frame of Peak}. Columns: \code{rt}, \code{from}, \code{to}, \code{intensity}, \code{integral}, \code{n_points}.
}
\section{Warning}{
Needs the whole file. A sample opened from a URL gives a remote_file error; open it from a path or bytes.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#find_transitions}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  fixtures <- "https://raw.githubusercontent.com/phenological/quantion/main/core/tests/fixtures"
  url <- paste0(fixtures, "/srm.ion")
  data <- curl::curl_fetch_memory(url)$content
  sample <- parse_ion(data)
  transitions <- data.frame(id = "compound_1", precursor = 89.04768, product = 89.04768,
                            rt = 2.385, range = 0.1)
  peaks <- find_transitions(sample, transitions, options = find_transitions_options(threads = 2L))
  print(peaks)
}
}
}
\seealso{
\code{\link{find_transitions_options}()}, \code{\link{find_targets}()}
}
