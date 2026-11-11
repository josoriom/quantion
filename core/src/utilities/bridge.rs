use std::slice;

const _: () = assert!(cfg!(target_endian = "little"));

pub const QUANTION_BRIDGE_MAGIC: u32 = 0x42544E51;
pub const QUANTION_BRIDGE_LAYOUT_VERSION: u16 = 1;
pub const QUANTION_BRIDGE_HEADER_BYTES: u64 = 32;
pub const QUANTION_SECTION_ENTRY_BYTES: u64 = 24;

pub const QUANTION_PAYLOAD_SCANS: u16 = 1;
pub const QUANTION_PAYLOAD_ION_IMAGE: u16 = 2;
pub const QUANTION_PAYLOAD_PEAKS: u16 = 3;
pub const QUANTION_PAYLOAD_FEATURES: u16 = 4;

pub const QUANTION_PAYLOAD_FIT_RESULT: u16 = 6;
pub const QUANTION_PAYLOAD_CONSENSUS_FEATURES: u16 = 8;
pub const QUANTION_PAYLOAD_FOUND_FEATURES: u16 = 9;
pub const QUANTION_PAYLOAD_WARP_TARGETS: u16 = 10;
pub const QUANTION_PAYLOAD_MODEL_METADATA: u16 = 11;
pub const QUANTION_PAYLOAD_SCAN_METADATA: u16 = 12;
pub const QUANTION_PAYLOAD_FEATURE_ANNOTATIONS: u16 = 13;
pub const QUANTION_PAYLOAD_FILE_ANNOTATIONS: u16 = 14;
pub const QUANTION_PAYLOAD_COMPOUND_MATCHES: u16 = 15;

pub const QUANTION_ELEMENT_F64: u32 = 1;
pub const QUANTION_ELEMENT_U32: u32 = 2;
pub const QUANTION_ELEMENT_U64: u32 = 3;
pub const QUANTION_ELEMENT_U8: u32 = 4;

pub const QUANTION_SECTION_POINT_STARTS: u32 = 1;
pub const QUANTION_SECTION_MZ: u32 = 2;
pub const QUANTION_SECTION_INTENSITY: u32 = 3;
pub const QUANTION_SECTION_RT: u32 = 4;
pub const QUANTION_SECTION_RT_SECONDS: u32 = 5;
pub const QUANTION_SECTION_BASE_PEAK_MZ: u32 = 6;
pub const QUANTION_SECTION_SELECTED_ION_MZ: u32 = 7;
pub const QUANTION_SECTION_BASE_PEAK_INT: u32 = 8;
pub const QUANTION_SECTION_TOTAL_ION_CURRENT: u32 = 9;
pub const QUANTION_SECTION_MS_LEVEL: u32 = 10;
pub const QUANTION_SECTION_POLARITY: u32 = 11;
pub const QUANTION_SECTION_POSITION_X: u32 = 12;
pub const QUANTION_SECTION_POSITION_Y: u32 = 13;
pub const QUANTION_SECTION_POSITION_Z: u32 = 14;

pub const QUANTION_SECTION_IMAGE_SHAPE: u32 = 15;
pub const QUANTION_SECTION_IMAGE_DATA: u32 = 16;
pub const QUANTION_SECTION_IMAGE_COUNTS: u32 = 17;

pub const QUANTION_SECTION_PEAK_FROM: u32 = 18;
pub const QUANTION_SECTION_PEAK_TO: u32 = 19;
pub const QUANTION_SECTION_PEAK_RT: u32 = 20;
pub const QUANTION_SECTION_PEAK_INTEGRAL: u32 = 21;
pub const QUANTION_SECTION_PEAK_INTENSITY: u32 = 22;
pub const QUANTION_SECTION_PEAK_POINT_COUNT: u32 = 23;

pub const QUANTION_SECTION_FEATURE_MZ: u32 = 26;
pub const QUANTION_SECTION_FEATURE_RT: u32 = 27;
pub const QUANTION_SECTION_FEATURE_FROM: u32 = 28;
pub const QUANTION_SECTION_FEATURE_TO: u32 = 29;
pub const QUANTION_SECTION_FEATURE_INTENSITY: u32 = 30;
pub const QUANTION_SECTION_FEATURE_INTEGRAL: u32 = 31;
pub const QUANTION_SECTION_FEATURE_POINT_COUNT: u32 = 32;

pub const QUANTION_SECTION_FIT_SHAPE: u32 = 46;
pub const QUANTION_SECTION_FIT_HEIGHT: u32 = 47;
pub const QUANTION_SECTION_FIT_CENTER: u32 = 48;
pub const QUANTION_SECTION_FIT_FWHM: u32 = 49;
pub const QUANTION_SECTION_FIT_TAIL: u32 = 50;
pub const QUANTION_SECTION_FIT_R2: u32 = 51;

pub const QUANTION_SECTION_CONSENSUS_MZ: u32 = 62;
pub const QUANTION_SECTION_CONSENSUS_RT: u32 = 63;
pub const QUANTION_SECTION_CONSENSUS_FROM: u32 = 64;
pub const QUANTION_SECTION_CONSENSUS_TO: u32 = 65;
pub const QUANTION_SECTION_CONSENSUS_INTENSITY: u32 = 66;
pub const QUANTION_SECTION_CONSENSUS_INTEGRAL: u32 = 67;
pub const QUANTION_SECTION_CONSENSUS_FREQUENCY: u32 = 68;

pub const QUANTION_SECTION_FOUND_ID_STARTS: u32 = 69;
pub const QUANTION_SECTION_FOUND_ID_BYTES: u32 = 70;
pub const QUANTION_SECTION_FOUND_MZ: u32 = 71;
pub const QUANTION_SECTION_FOUND_RT: u32 = 72;
pub const QUANTION_SECTION_FOUND_FROM: u32 = 73;
pub const QUANTION_SECTION_FOUND_TO: u32 = 74;
pub const QUANTION_SECTION_FOUND_INTENSITY: u32 = 75;
pub const QUANTION_SECTION_FOUND_INTEGRAL: u32 = 76;
pub const QUANTION_SECTION_FOUND_POINT_COUNT: u32 = 77;
pub const QUANTION_SECTION_FOUND_NOISE: u32 = 78;

pub const QUANTION_SECTION_WARP_TARGET_ID_STARTS: u32 = 79;
pub const QUANTION_SECTION_WARP_TARGET_ID_BYTES: u32 = 80;
pub const QUANTION_SECTION_WARP_TARGET_MZ: u32 = 81;
pub const QUANTION_SECTION_WARP_TARGET_RT: u32 = 82;

pub const QUANTION_SECTION_MODEL_METADATA_KEY_STARTS: u32 = 83;
pub const QUANTION_SECTION_MODEL_METADATA_KEY_BYTES: u32 = 84;
pub const QUANTION_SECTION_MODEL_METADATA_VALUE_STARTS: u32 = 85;
pub const QUANTION_SECTION_MODEL_METADATA_VALUE_BYTES: u32 = 86;
pub const QUANTION_SECTION_CONSENSUS_POINT_COUNT: u32 = 87;

pub const QUANTION_SECTION_SCAN_ARRAY_STARTS: u32 = 88;
pub const QUANTION_SECTION_SCAN_ARRAY_VALUES: u32 = 89;
pub const QUANTION_SECTION_SCAN_INDEX: u32 = 90;

pub const QUANTION_SECTION_SCAN_METADATA_PATH_STARTS: u32 = 91;
pub const QUANTION_SECTION_SCAN_METADATA_PATH_BYTES: u32 = 92;
pub const QUANTION_SECTION_SCAN_METADATA_KIND_STARTS: u32 = 93;
pub const QUANTION_SECTION_SCAN_METADATA_KIND_BYTES: u32 = 94;
pub const QUANTION_SECTION_SCAN_METADATA_ACCESSION_STARTS: u32 = 95;
pub const QUANTION_SECTION_SCAN_METADATA_ACCESSION_BYTES: u32 = 96;
pub const QUANTION_SECTION_SCAN_METADATA_ACCESSION_MISSING: u32 = 97;
pub const QUANTION_SECTION_SCAN_METADATA_NAME_STARTS: u32 = 98;
pub const QUANTION_SECTION_SCAN_METADATA_NAME_BYTES: u32 = 99;
pub const QUANTION_SECTION_SCAN_METADATA_VALUE_STARTS: u32 = 100;
pub const QUANTION_SECTION_SCAN_METADATA_VALUE_BYTES: u32 = 101;
pub const QUANTION_SECTION_SCAN_METADATA_VALUE_MISSING: u32 = 102;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_STARTS: u32 = 103;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_BYTES: u32 = 104;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_MISSING: u32 = 105;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_STARTS: u32 = 106;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_BYTES: u32 = 107;
pub const QUANTION_SECTION_SCAN_METADATA_UNIT_ACCESSION_MISSING: u32 = 108;

pub const QUANTION_SECTION_FEATURE_ANNOTATION_MZ: u32 = 109;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_RT: u32 = 110;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_STARTS: u32 = 111;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_BYTES: u32 = 112;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_LABEL_MISSING: u32 = 113;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_ID_STARTS: u32 = 114;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_ID_BYTES: u32 = 115;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_ID_MISSING: u32 = 116;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_STARTS: u32 = 117;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_BYTES: u32 = 118;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_DATABASE_MISSING: u32 = 119;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_FREQUENCY: u32 = 120;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_SCORE: u32 = 121;
pub const QUANTION_SECTION_FEATURE_ANNOTATION_COSINE: u32 = 122;
pub const QUANTION_SECTION_FILE_ANNOTATION_FEATURE: u32 = 123;
pub const QUANTION_SECTION_FILE_ANNOTATION_FILE: u32 = 124;
pub const QUANTION_SECTION_FILE_ANNOTATION_SCAN_INDEX: u32 = 125;
pub const QUANTION_SECTION_COMPOUND_MATCH_FEATURE: u32 = 126;
pub const QUANTION_SECTION_COMPOUND_MATCH_FILE: u32 = 127;
pub const QUANTION_SECTION_COMPOUND_MATCH_LABEL_STARTS: u32 = 128;
pub const QUANTION_SECTION_COMPOUND_MATCH_LABEL_BYTES: u32 = 129;
pub const QUANTION_SECTION_COMPOUND_MATCH_ID_STARTS: u32 = 130;
pub const QUANTION_SECTION_COMPOUND_MATCH_ID_BYTES: u32 = 131;
pub const QUANTION_SECTION_COMPOUND_MATCH_DATABASE_STARTS: u32 = 132;
pub const QUANTION_SECTION_COMPOUND_MATCH_DATABASE_BYTES: u32 = 133;
pub const QUANTION_SECTION_COMPOUND_MATCH_SCORE: u32 = 134;
pub const QUANTION_SECTION_COMPOUND_MATCH_COSINE: u32 = 135;
pub const QUANTION_SECTION_COMPOUND_MATCH_MZ_STARTS: u32 = 136;
pub const QUANTION_SECTION_COMPOUND_MATCH_MZ_VALUES: u32 = 137;
pub const QUANTION_SECTION_COMPOUND_MATCH_INTENSITY_STARTS: u32 = 138;
pub const QUANTION_SECTION_COMPOUND_MATCH_INTENSITY_VALUES: u32 = 139;
pub const QUANTION_SECTION_COMPOUND_MATCH_METADATA_NAME_STARTS: u32 = 140;
pub const QUANTION_SECTION_COMPOUND_MATCH_METADATA_NAME_BYTES: u32 = 141;
pub const QUANTION_SECTION_COMPOUND_MATCH_METADATA_VALUE_STARTS: u32 = 142;
pub const QUANTION_SECTION_COMPOUND_MATCH_METADATA_VALUE_BYTES: u32 = 143;

const ALIGNMENT: u64 = 8;

fn size_of_element(element_type: u32) -> Option<u64> {
    match element_type {
        QUANTION_ELEMENT_F64 => Some(8),
        QUANTION_ELEMENT_U32 => Some(4),
        QUANTION_ELEMENT_U64 => Some(8),
        QUANTION_ELEMENT_U8 => Some(1),
        _ => None,
    }
}

fn round_up(value: u64) -> Option<u64> {
    value
        .checked_add(ALIGNMENT - 1)
        .map(|sum| sum & !(ALIGNMENT - 1))
}

#[derive(Clone, Copy)]
struct Section {
    id: u32,
    element_type: u32,
    element_count: u64,
    byte_offset: u64,
    byte_length: u64,
}

pub struct BridgeBuilder {
    payload_kind: u16,
    record_count: u64,
    sections: Vec<Section>,
}

impl BridgeBuilder {
    pub fn new(payload_kind: u16, record_count: u64) -> Self {
        Self {
            payload_kind,
            record_count,
            sections: Vec::new(),
        }
    }

    pub fn add_section(&mut self, id: u32, element_type: u32, element_count: u64) {
        self.sections.push(Section {
            id,
            element_type,
            element_count,
            byte_offset: 0,
            byte_length: 0,
        });
    }

    pub fn build(mut self) -> Option<Bridge> {
        let section_count = u32::try_from(self.sections.len()).ok()?;
        let table_bytes = QUANTION_SECTION_ENTRY_BYTES.checked_mul(u64::from(section_count))?;
        let mut next_offset = QUANTION_BRIDGE_HEADER_BYTES.checked_add(table_bytes)?;

        for section in self.sections.iter_mut() {
            let element_size = size_of_element(section.element_type)?;
            let byte_length = section.element_count.checked_mul(element_size)?;
            section.byte_offset = next_offset;
            section.byte_length = byte_length;
            next_offset = round_up(next_offset.checked_add(byte_length)?)?;
        }

        let total_bytes = next_offset;
        let total = usize::try_from(total_bytes).ok()?;
        let mut bytes = vec![0u8; total];

        bytes[0..4].copy_from_slice(&QUANTION_BRIDGE_MAGIC.to_le_bytes());
        bytes[4..6].copy_from_slice(&QUANTION_BRIDGE_LAYOUT_VERSION.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.payload_kind.to_le_bytes());
        bytes[8..12].copy_from_slice(&section_count.to_le_bytes());
        bytes[12..16].copy_from_slice(&(QUANTION_BRIDGE_HEADER_BYTES as u32).to_le_bytes());
        bytes[16..24].copy_from_slice(&total_bytes.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.record_count.to_le_bytes());

        for (index, section) in self.sections.iter().enumerate() {
            let start = (QUANTION_BRIDGE_HEADER_BYTES as usize)
                + index * (QUANTION_SECTION_ENTRY_BYTES as usize);
            bytes[start..start + 4].copy_from_slice(&section.id.to_le_bytes());
            bytes[start + 4..start + 8].copy_from_slice(&section.element_type.to_le_bytes());
            bytes[start + 8..start + 16].copy_from_slice(&section.byte_offset.to_le_bytes());
            bytes[start + 16..start + 24].copy_from_slice(&section.byte_length.to_le_bytes());
        }

        Some(Bridge {
            bytes,
            sections: self.sections,
        })
    }
}

pub struct Bridge {
    bytes: Vec<u8>,
    sections: Vec<Section>,
}

impl Bridge {
    fn find_section(&self, id: u32, element_type: u32) -> Section {
        let section = self
            .sections
            .iter()
            .find(|section| section.id == id)
            .expect("bridge section was never declared");
        assert_eq!(
            section.element_type, element_type,
            "bridge section written with the wrong element type"
        );
        *section
    }

    fn copy_into(&mut self, byte_offset: u64, source: &[u8]) {
        let start = byte_offset as usize;
        self.bytes[start..start + source.len()].copy_from_slice(source);
    }

    pub fn write_f64_at(&mut self, id: u32, element_offset: u64, values: &[f64]) {
        let section = self.find_section(id, QUANTION_ELEMENT_F64);
        let end = element_offset + values.len() as u64;
        assert!(
            end <= section.element_count,
            "bridge f64 write out of range"
        );
        let source = unsafe {
            slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
        };
        self.copy_into(section.byte_offset + element_offset * 8, source);
    }

    pub fn write_f64_section(&mut self, id: u32, values: &[f64]) {
        let section = self.find_section(id, QUANTION_ELEMENT_F64);
        assert_eq!(
            section.element_count,
            values.len() as u64,
            "bridge f64 section written with the wrong element count"
        );
        self.write_f64_at(id, 0, values);
    }

    pub fn write_u32_section(&mut self, id: u32, values: &[u32]) {
        let section = self.find_section(id, QUANTION_ELEMENT_U32);
        assert_eq!(
            section.element_count,
            values.len() as u64,
            "bridge u32 section written with the wrong element count"
        );
        let source = unsafe {
            slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
        };
        self.copy_into(section.byte_offset, source);
    }

    pub fn write_u64_section(&mut self, id: u32, values: &[u64]) {
        let section = self.find_section(id, QUANTION_ELEMENT_U64);
        assert_eq!(
            section.element_count,
            values.len() as u64,
            "bridge u64 section written with the wrong element count"
        );
        let source = unsafe {
            slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
        };
        self.copy_into(section.byte_offset, source);
    }

    pub fn write_u8_section(&mut self, id: u32, values: &[u8]) {
        let section = self.find_section(id, QUANTION_ELEMENT_U8);
        assert_eq!(
            section.element_count,
            values.len() as u64,
            "bridge u8 section written with the wrong element count"
        );
        self.copy_into(section.byte_offset, values);
    }

    pub fn write_text_section(&mut self, id: u32, values: &[&str]) {
        let section = self.find_section(id, QUANTION_ELEMENT_U8);
        let mut at = section.byte_offset;
        for text in values {
            let bytes = text.as_bytes();
            self.copy_into(at, bytes);
            at += bytes.len() as u64;
        }
        assert_eq!(
            at - section.byte_offset,
            section.byte_length,
            "bridge text section written with the wrong byte count"
        );
    }

    pub fn into_bytes(self) -> Box<[u8]> {
        debug_assert_eq!(
            self.bytes.capacity(),
            self.bytes.len(),
            "bridge allocation must not carry spare capacity"
        );
        self.bytes.into_boxed_slice()
    }
}

pub enum Column<'a> {
    Numbers {
        id: u32,
        values: &'a [f64],
    },
    Counts {
        id: u32,
        values: &'a [u32],
    },
    Text {
        starts_id: u32,
        bytes_id: u32,
        values: &'a [&'a str],
    },
    OptionalText {
        starts_id: u32,
        bytes_id: u32,
        missing_id: u32,
        values: &'a [Option<&'a str>],
    },
    NumberLists {
        starts_id: u32,
        values_id: u32,
        values: &'a [&'a [f64]],
    },
}

pub fn build_record_bridge(
    payload_kind: u16,
    record_count: u64,
    columns: &[Column],
) -> Option<Box<[u8]>> {
    let mut builder = BridgeBuilder::new(payload_kind, record_count);
    for column in columns {
        match column {
            Column::Numbers { id, values } => {
                builder.add_section(*id, QUANTION_ELEMENT_F64, values.len() as u64)
            }
            Column::Counts { id, values } => {
                builder.add_section(*id, QUANTION_ELEMENT_U32, values.len() as u64)
            }
            Column::Text {
                starts_id,
                bytes_id,
                values,
            } => {
                let mut total: u64 = 0;
                for text in values.iter() {
                    total = total.checked_add(text.len() as u64)?;
                }
                builder.add_section(*starts_id, QUANTION_ELEMENT_U64, values.len() as u64 + 1);
                builder.add_section(*bytes_id, QUANTION_ELEMENT_U8, total);
            }
            Column::OptionalText {
                starts_id,
                bytes_id,
                missing_id,
                values,
            } => {
                let mut total: u64 = 0;
                for text in values.iter().flatten() {
                    total = total.checked_add(text.len() as u64)?;
                }
                builder.add_section(*starts_id, QUANTION_ELEMENT_U64, values.len() as u64 + 1);
                builder.add_section(*bytes_id, QUANTION_ELEMENT_U8, total);
                builder.add_section(*missing_id, QUANTION_ELEMENT_U8, values.len() as u64);
            }
            Column::NumberLists {
                starts_id,
                values_id,
                values,
            } => {
                let mut total: u64 = 0;
                for list in values.iter() {
                    total = total.checked_add(list.len() as u64)?;
                }
                builder.add_section(*starts_id, QUANTION_ELEMENT_U64, values.len() as u64 + 1);
                builder.add_section(*values_id, QUANTION_ELEMENT_F64, total);
            }
        }
    }
    let mut bridge = builder.build()?;

    for column in columns {
        match column {
            Column::Numbers { id, values } => bridge.write_f64_section(*id, values),
            Column::Counts { id, values } => bridge.write_u32_section(*id, values),
            Column::Text {
                starts_id,
                bytes_id,
                values,
            } => {
                let mut starts = Vec::with_capacity(values.len() + 1);
                let mut next: u64 = 0;
                starts.push(0u64);
                for text in values.iter() {
                    next += text.len() as u64;
                    starts.push(next);
                }
                bridge.write_u64_section(*starts_id, &starts);
                bridge.write_text_section(*bytes_id, values);
            }
            Column::OptionalText {
                starts_id,
                bytes_id,
                missing_id,
                values,
            } => {
                let texts: Vec<&str> = values.iter().map(|text| text.unwrap_or_default()).collect();
                let missing: Vec<u8> = values.iter().map(|text| u8::from(text.is_none())).collect();
                let mut starts = Vec::with_capacity(texts.len() + 1);
                let mut next: u64 = 0;
                starts.push(0u64);
                for text in &texts {
                    next += text.len() as u64;
                    starts.push(next);
                }
                bridge.write_u64_section(*starts_id, &starts);
                bridge.write_text_section(*bytes_id, &texts);
                bridge.write_u8_section(*missing_id, &missing);
            }
            Column::NumberLists {
                starts_id,
                values_id,
                values,
            } => {
                let mut starts = Vec::with_capacity(values.len() + 1);
                let mut next: u64 = 0;
                starts.push(0u64);
                for list in values.iter() {
                    next += list.len() as u64;
                    starts.push(next);
                }
                let flat: Vec<f64> = values
                    .iter()
                    .flat_map(|list| list.iter().copied())
                    .collect();
                bridge.write_u64_section(*starts_id, &starts);
                bridge.write_f64_section(*values_id, &flat);
            }
        }
    }
    Some(bridge.into_bytes())
}
