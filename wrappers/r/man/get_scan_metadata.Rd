\name{get_scan_metadata}
\alias{get_scan_metadata}
\title{Read the params of one scan}
\description{
Read the attributes, cvParams and userParams of one scan, one row each, parents before children. Find the scan with get_scans, then pass its index.
}
\usage{
get_scan_metadata(file, index, options = get_scan_metadata_options())
}
\arguments{
  \item{file}{The sample to read, from parse_ion or parse_mzml. Type: \code{SampleFile}.}
  \item{index}{Position of the scan in the spectrum list, from 0: the index of a scan from get_scans. Type: \code{numeric(1)}.}
  \item{options}{Settings from \code{\link{get_scan_metadata_options}()}: \code{tags = character(0)}.
    \href{https://phenological.github.io/quantion/?language=r#GetScanMetadataOptions}{GetScanMetadataOptions documentation}
  }
}
\value{
One row per attribute or param, in the order of the mzML tree. Values are text. A field the file does not give is missing: None in Rust and Python, null in JS, NA in R. Type: \code{data.frame of ScanParam}. Columns: \code{path}, \code{kind}, \code{accession}, \code{name}, \code{value}, \code{unit}, \code{unit_accession}.
}
\section{Documentation}{
\url{https://phenological.github.io/quantion/?language=r#get_scan_metadata}
}
\examples{
\donttest{
if (curl::has_internet()) {
  library(quantion)

  url <- "https://raw.githubusercontent.com/phenological/ion-files/main/data/aa_p21_LTR_20_b1_w20.ion"
  file <- parse_ion(url)
  scans <- get_scans(file, rt = c(2.3, 2.4))
  rows <- get_scan_metadata(file, scans$index[1])
  only_scan <- get_scan_metadata(file, scans$index[1], get_scan_metadata_options(tags = "scan"))
  print(rows[1, ])
  print(nrow(only_scan))
}
}
}
