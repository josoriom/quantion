use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ionic::{IonReader, mzml::structs::MzML};

use crate::{
    error::{ErrorKind, QuantionError},
    utilities::{
        calculate_eic::EicReader,
        ion::{ScanSource, ScanSummary},
        parse_ion::{PrefetchRanges, is_url},
    },
};

pub(crate) enum FileSource {
    Full(Box<MzML>),
    Lazy(Box<IonReader>),
    Remote(Box<IonReader>),
}

/// **SampleFile** · Inputs
///
/// An opened sample from parse_ion or parse_mzml. Free it with dispose when you are done.
///
/// [Documentation](https://phenological.github.io/quantion/?language=rust#SampleFile)
pub struct SampleFile {
    pub(crate) source: FileSource,
    remote: Option<Arc<dyn PrefetchRanges>>,
}

impl SampleFile {
    pub(crate) fn new(source: FileSource) -> Self {
        Self {
            source,
            remote: None,
        }
    }

    pub(crate) fn with_remote(source: FileSource, remote: Option<Arc<dyn PrefetchRanges>>) -> Self {
        Self { source, remote }
    }

    pub(crate) fn prefetch(&self, ranges: Vec<ionic::source::ByteRange>) -> ionic::IonResult<()> {
        match &self.remote {
            Some(remote) => remote.prefetch(ranges),
            None => Ok(()),
        }
    }

    pub(crate) fn is_remote(&self) -> bool {
        self.remote.is_some()
    }

    pub(crate) fn reader(&mut self) -> EicReader<'_> {
        match &mut self.source {
            FileSource::Full(mzml) => EicReader::Mzml(mzml.as_mut()),
            FileSource::Lazy(ion) => EicReader::Ion(ion.as_mut()),
            FileSource::Remote(ion) => EicReader::Ion(ion.as_mut()),
        }
    }

    pub(crate) fn is_ion(&self) -> bool {
        !matches!(self.source, FileSource::Full(_))
    }

    pub(crate) fn with_mzml<T>(
        &mut self,
        function: &'static str,
        work: impl FnOnce(&MzML) -> Result<T, QuantionError>,
    ) -> Result<T, QuantionError> {
        match &mut self.source {
            FileSource::Full(mzml) => work(mzml.as_ref()),
            FileSource::Lazy(ion) => {
                let mzml = ion.to_mzml().map_err(|error| {
                    QuantionError::new(
                        ErrorKind::ParseFailed,
                        function,
                        format!("the file could not be read: {error}"),
                    )
                })?;
                work(&mzml)
            }
            FileSource::Remote(_) => Err(QuantionError::remote_file(function)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MzmlInput {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

impl From<&str> for MzmlInput {
    fn from(text: &str) -> Self {
        MzmlInput::Path(PathBuf::from(text))
    }
}

impl From<String> for MzmlInput {
    fn from(text: String) -> Self {
        MzmlInput::Path(PathBuf::from(text))
    }
}

impl From<&Path> for MzmlInput {
    fn from(path: &Path) -> Self {
        MzmlInput::Path(path.to_path_buf())
    }
}

impl From<PathBuf> for MzmlInput {
    fn from(path: PathBuf) -> Self {
        MzmlInput::Path(path)
    }
}

impl From<Vec<u8>> for MzmlInput {
    fn from(bytes: Vec<u8>) -> Self {
        MzmlInput::Bytes(bytes)
    }
}

impl From<&[u8]> for MzmlInput {
    fn from(bytes: &[u8]) -> Self {
        MzmlInput::Bytes(bytes.to_vec())
    }
}

pub(crate) fn is_url_path(path: &Path) -> bool {
    path.to_str().is_some_and(is_url)
}

impl ScanSource for SampleFile {
    fn for_each_summary(&mut self, callback: &mut dyn FnMut(usize, ScanSummary)) {
        match &mut self.source {
            FileSource::Full(mzml) => mzml.for_each_summary(callback),
            FileSource::Lazy(ion) | FileSource::Remote(ion) => ion.for_each_summary(callback),
        }
    }

    fn load_scan(&mut self, index: usize, mz: &mut Vec<f64>, intensity: &mut Vec<f64>) -> bool {
        match &mut self.source {
            FileSource::Full(mzml) => mzml.load_scan(index, mz, intensity),
            FileSource::Lazy(ion) | FileSource::Remote(ion) => ion.load_scan(index, mz, intensity),
        }
    }

    fn load_array(&mut self, index: usize, accession: u32, values: &mut Vec<f64>) -> bool {
        match &mut self.source {
            FileSource::Full(mzml) => mzml.load_array(index, accession, values),
            FileSource::Lazy(ion) | FileSource::Remote(ion) => {
                ion.load_array(index, accession, values)
            }
        }
    }

    fn load_metadata(&mut self, index: usize) -> ionic::IonResult<Vec<crate::files::ScanParam>> {
        match &mut self.source {
            FileSource::Full(mzml) => mzml.load_metadata(index),
            FileSource::Lazy(ion) | FileSource::Remote(ion) => ion.load_metadata(index),
        }
    }
}
