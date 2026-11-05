use ionic::mzml::structs::{
    BinaryDataArray, CvParam, IsolationWindow, Precursor, PrecursorList, Product, ReferenceableParamGroupRef, Scan, ScanList,
    Spectrum, SpectrumDescription, UserParam,
};

use crate::files::{ScanParam, ScanParamKind};

pub(crate) const SCAN_METADATA_TAGS: [&str; 17] = [
    "spectrum",
    "referenceableParamGroupRef",
    "spectrumDescription",
    "scanList",
    "scan",
    "scanWindowList",
    "scanWindow",
    "precursorList",
    "precursor",
    "isolationWindow",
    "selectedIonList",
    "selectedIon",
    "activation",
    "productList",
    "product",
    "binaryDataArrayList",
    "binaryDataArray",
];

pub(crate) fn get_tag(path: &str) -> &str {
    let last = path.rsplit('/').next().unwrap_or(path);
    last.split('[').next().unwrap_or(last)
}

pub(crate) fn get_scan_params(spectrum: &Spectrum) -> Vec<ScanParam> {
    let mut rows = Rows::default();
    let path = "spectrum".to_string();
    let index = spectrum.index.map(|index| index.to_string());
    let length = spectrum.default_array_length.map(|length| length.to_string());
    rows.add_attributes(
        &path,
        &[
            ("index", index.as_deref()),
            ("id", Some(spectrum.id.as_str())),
            ("defaultArrayLength", length.as_deref()),
            ("nativeID", spectrum.native_id.as_deref()),
            ("dataProcessingRef", spectrum.data_processing_ref.as_deref()),
            ("sourceFileRef", spectrum.source_file_ref.as_deref()),
            ("spotID", spectrum.spot_id.as_deref()),
        ],
    );
    rows.add_params(&path, &spectrum.referenceable_param_group_refs, &spectrum.cv_params, &spectrum.user_params);
    if let Some(description) = &spectrum.spectrum_description {
        rows.add_description(&format!("{path}/spectrumDescription"), description);
    }
    rows.add_scan_list(&path, spectrum.scan_list.as_ref());
    rows.add_precursors(&path, spectrum.precursor_list.as_ref());
    if let Some(list) = &spectrum.product_list {
        let list_path = format!("{path}/productList");
        rows.add_params(&list_path, &[], &list.cv_params, &list.user_params);
        for (position, product) in list.products.iter().enumerate() {
            rows.add_product(&format!("{list_path}/product[{position}]"), product);
        }
    }
    if let Some(list) = &spectrum.binary_data_array_list {
        let list_path = format!("{path}/binaryDataArrayList");
        for (position, array) in list.binary_data_arrays.iter().enumerate() {
            rows.add_binary_array(&format!("{list_path}/binaryDataArray[{position}]"), array);
        }
    }
    rows.rows
}

#[derive(Default)]
struct Rows {
    rows: Vec<ScanParam>,
}

impl Rows {
    fn add_attributes(&mut self, path: &str, attributes: &[(&str, Option<&str>)]) {
        for (name, value) in attributes {
            if let Some(value) = get_text(*value) {
                self.rows.push(ScanParam {
                    path: path.to_string(),
                    kind: ScanParamKind::ATTRIBUTE,
                    accession: None,
                    name: name.to_string(),
                    value: Some(value),
                    unit: None,
                    unit_accession: None,
                });
            }
        }
    }

    fn add_params(&mut self, path: &str, groups: &[ReferenceableParamGroupRef], cv_params: &[CvParam], user_params: &[UserParam]) {
        for (position, group) in groups.iter().enumerate() {
            let group_path = format!("{path}/referenceableParamGroupRef[{position}]");
            self.add_attributes(&group_path, &[("ref", Some(group.r#ref.as_str()))]);
        }
        for param in cv_params {
            self.rows.push(ScanParam {
                path: path.to_string(),
                kind: ScanParamKind::CV,
                accession: get_text(param.accession.as_deref()),
                name: param.name.clone(),
                value: get_text(param.value.as_deref()),
                unit: get_text(param.unit_name.as_deref()),
                unit_accession: get_text(param.unit_accession.as_deref()),
            });
        }
        for param in user_params {
            self.rows.push(ScanParam {
                path: path.to_string(),
                kind: ScanParamKind::USER,
                accession: None,
                name: param.name.clone(),
                value: get_text(param.value.as_deref()),
                unit: get_text(param.unit_name.as_deref()),
                unit_accession: get_text(param.unit_accession.as_deref()),
            });
        }
    }

    fn add_description(&mut self, path: &str, description: &SpectrumDescription) {
        self.add_params(path, &description.referenceable_param_group_refs, &description.cv_params, &description.user_params);
        self.add_scan_list(path, description.scan_list.as_ref());
        self.add_precursors(path, description.precursor_list.as_ref());
    }

    fn add_scan_list(&mut self, parent: &str, list: Option<&ScanList>) {
        let Some(list) = list else {
            return;
        };
        let list_path = format!("{parent}/scanList");
        self.add_params(&list_path, &list.referenceable_param_group_refs, &list.cv_params, &list.user_params);
        for (position, scan) in list.scans.iter().enumerate() {
            self.add_scan(&format!("{list_path}/scan[{position}]"), scan);
        }
    }

    fn add_scan(&mut self, path: &str, scan: &Scan) {
        self.add_attributes(
            path,
            &[
                ("instrumentConfigurationRef", scan.instrument_configuration_ref.as_deref()),
                ("externalSpectrumID", scan.external_spectrum_id.as_deref()),
                ("sourceFileRef", scan.source_file_ref.as_deref()),
                ("spectrumRef", scan.spectrum_ref.as_deref()),
            ],
        );
        self.add_params(path, &scan.referenceable_param_group_refs, &scan.cv_params, &scan.user_params);
        if let Some(list) = &scan.scan_window_list {
            let list_path = format!("{path}/scanWindowList");
            for (position, window) in list.scan_windows.iter().enumerate() {
                self.add_params(&format!("{list_path}/scanWindow[{position}]"), &[], &window.cv_params, &window.user_params);
            }
        }
    }

    fn add_precursors(&mut self, parent: &str, list: Option<&PrecursorList>) {
        let Some(list) = list else {
            return;
        };
        let list_path = format!("{parent}/precursorList");
        self.add_params(&list_path, &[], &list.cv_params, &list.user_params);
        for (position, precursor) in list.precursors.iter().enumerate() {
            self.add_precursor(&format!("{list_path}/precursor[{position}]"), precursor);
        }
    }

    fn add_precursor(&mut self, path: &str, precursor: &Precursor) {
        self.add_attributes(
            path,
            &[
                ("spectrumRef", precursor.spectrum_ref.as_deref()),
                ("sourceFileRef", precursor.source_file_ref.as_deref()),
                ("externalSpectrumID", precursor.external_spectrum_id.as_deref()),
            ],
        );
        self.add_isolation_window(path, precursor.isolation_window.as_ref());
        if let Some(list) = &precursor.selected_ion_list {
            let list_path = format!("{path}/selectedIonList");
            for (position, ion) in list.selected_ions.iter().enumerate() {
                let ion_path = format!("{list_path}/selectedIon[{position}]");
                self.add_params(&ion_path, &ion.referenceable_param_group_refs, &ion.cv_params, &ion.user_params);
            }
        }
        if let Some(activation) = &precursor.activation {
            let activation_path = format!("{path}/activation");
            self.add_params(&activation_path, &activation.referenceable_param_group_refs, &activation.cv_params, &activation.user_params);
        }
    }

    fn add_product(&mut self, path: &str, product: &Product) {
        self.add_attributes(
            path,
            &[
                ("spectrumRef", product.spectrum_ref.as_deref()),
                ("sourceFileRef", product.source_file_ref.as_deref()),
                ("externalSpectrumID", product.external_spectrum_id.as_deref()),
            ],
        );
        self.add_params(path, &[], &product.cv_params, &product.user_params);
        self.add_isolation_window(path, product.isolation_window.as_ref());
    }

    fn add_isolation_window(&mut self, parent: &str, window: Option<&IsolationWindow>) {
        if let Some(window) = window {
            let path = format!("{parent}/isolationWindow");
            self.add_params(&path, &window.referenceable_param_group_refs, &window.cv_params, &window.user_params);
        }
    }

    fn add_binary_array(&mut self, path: &str, array: &BinaryDataArray) {
        let array_length = array.array_length.map(|length| length.to_string());
        let encoded_length = array.encoded_length.map(|length| length.to_string());
        self.add_attributes(
            path,
            &[
                ("arrayLength", array_length.as_deref()),
                ("encodedLength", encoded_length.as_deref()),
                ("dataProcessingRef", array.data_processing_ref.as_deref()),
            ],
        );
        self.add_params(path, &array.referenceable_param_group_refs, &array.cv_params, &array.user_params);
    }
}

fn get_text(value: Option<&str>) -> Option<String> {
    value.filter(|text| !text.is_empty()).map(str::to_string)
}
