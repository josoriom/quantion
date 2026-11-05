\name{write_ion_options}
\alias{write_ion_options}
\title{Settings for write_ion}
\description{
Settings for write_ion.
}
\usage{
write_ion_options(out = NULL, overwrite = FALSE, level = 12L, f32_compress = FALSE,
                  headers = list(), block_size = 8L, mz_window = 20L)
}
\arguments{
  \item{out}{Path or URL to write to. Not given = return the bytes.}
  \item{overwrite}{Replace an existing file.}
  \item{level}{Compression level, 0 to 22.}
  \item{f32_compress}{Store intensities as 32-bit floats.}
  \item{block_size}{Size of each data block before compression, in MB. A whole number, 1 or more.}
  \item{mz_window}{Width of the m/z windows the spectra are split into, in Da, so a query reads only the windows it needs. A whole number, 0 or more; 0 keeps each spectrum whole.}
  \item{headers}{Extra HTTP headers. Only with a URL.}
}
\value{
An options object for \code{\link{write_ion}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#WriteIonOptions}
}
\examples{
write_ion_options(out = "sample.ion", level = 12L)
}
\seealso{
\code{\link{write_ion}()}
}
