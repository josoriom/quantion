\name{get_scan_metadata_options}
\alias{get_scan_metadata_options}
\title{Settings for get_scan_metadata}
\description{
Settings for get_scan_metadata.
}
\usage{
get_scan_metadata_options(tags = character(0))
}
\arguments{
  \item{tags}{Which tags to keep, by name. A row is kept when the last tag of its path matches, so scan keeps spectrum/scanList/scan[0] but not its scanWindow. Empty keeps every tag. The names are: spectrum, referenceableParamGroupRef, spectrumDescription, scanList, scan, scanWindowList, scanWindow, precursorList, precursor, isolationWindow, selectedIonList, selectedIon, activation, productList, product, binaryDataArrayList, binaryDataArray.}
}
\value{
An options object for \code{\link{get_scan_metadata}()}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#GetScanMetadataOptions}
}
\examples{
get_scan_metadata_options(tags = c("scan", "selectedIon"))
}
\seealso{
\code{\link{get_scan_metadata}()}
}
