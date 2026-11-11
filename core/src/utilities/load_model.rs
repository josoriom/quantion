use std::{
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

use runnet::Tensor;

use crate::utilities::parse_ion::is_url;

#[derive(Debug, Clone)]
pub enum ModelInput {
    Path(PathBuf),
    Url(String),
    Bytes(Vec<u8>),
}

impl From<&str> for ModelInput {
    fn from(text: &str) -> Self {
        if is_url(text) {
            ModelInput::Url(text.to_string())
        } else {
            ModelInput::Path(PathBuf::from(text))
        }
    }
}

impl From<String> for ModelInput {
    fn from(text: String) -> Self {
        ModelInput::from(text.as_str())
    }
}

impl From<&String> for ModelInput {
    fn from(text: &String) -> Self {
        ModelInput::from(text.as_str())
    }
}

impl From<&Path> for ModelInput {
    fn from(path: &Path) -> Self {
        ModelInput::Path(path.to_path_buf())
    }
}

impl From<PathBuf> for ModelInput {
    fn from(path: PathBuf) -> Self {
        ModelInput::Path(path)
    }
}

impl From<Vec<u8>> for ModelInput {
    fn from(bytes: Vec<u8>) -> Self {
        ModelInput::Bytes(bytes)
    }
}

impl From<&[u8]> for ModelInput {
    fn from(bytes: &[u8]) -> Self {
        ModelInput::Bytes(bytes.to_vec())
    }
}

#[derive(Clone, Debug)]
pub enum ModelError {
    NotRead(String),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelError::NotRead(reason) => write!(f, "the model was not read: {}", reason),
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Clone, Debug)]
pub struct Model {
    onnx: Arc<runnet::Model>,
}

pub fn load_model(input: impl Into<ModelInput>) -> Result<Model, ModelError> {
    let onnx = match input.into() {
        ModelInput::Path(path) => runnet::Model::load(path),
        ModelInput::Url(url) => runnet::Model::from_bytes(&fetch_model(&url)?),
        ModelInput::Bytes(bytes) => runnet::Model::from_bytes(&bytes),
    }
    .map_err(|error| ModelError::NotRead(error.to_string()))?;
    Ok(Model {
        onnx: Arc::new(onnx),
    })
}

impl Model {
    pub fn get_keys(&self) -> &[String] {
        self.onnx.get_keys()
    }

    pub fn get_values(&self) -> &[String] {
        self.onnx.get_values()
    }

    pub fn get_value(&self, key: &str) -> Option<&str> {
        self.onnx.get_value(key)
    }

    pub(crate) fn input_names(&self) -> &[String] {
        self.onnx.input_names()
    }

    pub(crate) fn run(&self, inputs: Vec<Tensor>) -> Result<Vec<Tensor>, runnet::Error> {
        self.onnx.run(inputs)
    }
}

#[cfg(all(
    feature = "http",
    not(all(target_arch = "wasm32", not(target_os = "wasi")))
))]
fn fetch_model(url: &str) -> Result<Vec<u8>, ModelError> {
    ureq::get(url)
        .call()
        .map_err(|error| ModelError::NotRead(format!("GET {url} failed: {error}")))?
        .body_mut()
        .read_to_vec()
        .map_err(|error| ModelError::NotRead(format!("reading {url} failed: {error}")))
}

#[cfg(not(all(
    feature = "http",
    not(all(target_arch = "wasm32", not(target_os = "wasi")))
)))]
fn fetch_model(url: &str) -> Result<Vec<u8>, ModelError> {
    Err(ModelError::NotRead(format!(
        "{url} is a URL; enable the http feature of quantion to load URLs"
    )))
}
