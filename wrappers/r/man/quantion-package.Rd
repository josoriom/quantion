\name{quantion-package}
\alias{quantion}
\alias{quantion-package}
\title{quantion}
\description{
R wrappers for the \code{quantion} library for LC-MS data processing.
}
\details{
\code{quantion} loads mzML or .ion files, extracts EICs, picks peaks, and detects features across one or many samples.

A \emph{sample} is a pointer to a parsed mzML held in memory, or to an .ion file read on demand. Its memory is freed by the garbage collector; \code{\link{dispose}()} frees it at once.

Every function with settings takes one \code{options} argument built by a helper named after it, such as \code{\link{find_targets_options}()}. Groups of settings nest through their own helpers: \code{\link{calculate_eic_options}()}, \code{\link{grid_options}()}, \code{\link{grouping_options}()} and \code{\link{find_peaks_options}()}.

Errors are conditions of class \code{quantion_error} with the fields \code{kind}, \code{code}, \code{function}, \code{argument} and \code{message}, the same in every quantion language.
}
\section{Files}{
\code{\link{parse_ion}()}, \code{\link{parse_mzml}()} - open a file.

\code{\link{write_ion}()}, \code{\link{write_mzml}()} - write a sample.

\code{\link{get_scans}()} - read scans by retention time or m/z.

\code{\link{calculate_eic}()} - extract an ion chromatogram.

\code{\link{get_ion_image}()} - build an ion image.

\code{\link{dispose}()} - free a sample, a model or a warp model.
}
\section{Traces}{
\code{\link{find_peaks}()} - find every peak in a trace.

\code{\link{get_peak}()} - find the one peak near a retention time.

\code{\link{fit_peak}()}, \code{\link{draw_peaks}()} - fit and draw peak curves.

\code{\link{find_noise_level}()}, \code{\link{calculate_baseline}()} - noise and baseline.
}
\section{Targets and features}{
\code{\link{find_targets}()}, \code{\link{find_transitions}()} - targeted peaks.

\code{\link{find_features}()}, \code{\link{match_features}()} - untargeted features.
}
\section{Models}{
\code{\link{load_model}()}, \code{\link{load_warp}()}, \code{\link{predict_rts}()}, \code{\link{calibrate}()}, \code{\link{get_keys}()}.
}
\keyword{package}
