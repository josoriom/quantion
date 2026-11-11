\name{get_ion_image}
\alias{get_ion_image}
\title{Build an ion image}
\description{
Build an imaging map of one m/z: summed intensity at each x, y position.
}
\usage{
get_ion_image(file, mz, options = get_ion_image_options())
}
\arguments{
  \item{file}{The sample to read. Must come from parse_ion. Type: \code{SampleFile}.}
  \item{mz}{The m/z to map (Da). Type: \code{numeric}.}
  \item{options}{Settings from \code{\link{get_ion_image_options}()}: \code{tolerance = 0.125}, \code{level = 1L}, \code{on_progress = NULL}.
    \href{https://phenological.github.io/quantion/?language=r#GetIonImageOptions}{GetIonImageOptions documentation}
  }
}
\value{
The image. Type: \code{IonImage list}. Fields: \code{width}, \code{height}, \code{min_x}, \code{min_y}, \code{min_z}, \code{max_z}, \code{data}, \code{counts}.
}
\section{Warning}{
Only Ionic files (.ion). The sample must come from parse_ion; a sample from parse_mzml gives an invalid_argument error.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#get_ion_image}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  sample <- parse_ion(url)
  image <- get_ion_image(sample, 260.103, options = get_ion_image_options(tolerance = 0.05))
  print(c(image$width, image$height))
}
}
}
\seealso{
\code{\link{get_ion_image_options}()}
}
