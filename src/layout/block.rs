use super::entry::LeafEntry;
use super::errors::LayoutError;
use super::header::Header;
use super::settings::{CrcConfig, CrcLocation, Endianness, Settings};
use crate::data::DataSource;

use indexmap::IndexMap;
use serde::Deserialize;

/// Mutable state tracked during recursive bytestream building
struct BuildState {
    buffer: Vec<u8>,
    offset: usize,
    padding_count: u32,
}

/// Immutable configuration for bytestream building
pub struct BuildConfig<'a> {
    pub endianness: &'a Endianness,
    pub padding: u8,
    pub strict: bool,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub settings: Settings,
    #[serde(flatten)]
    pub blocks: IndexMap<String, Block>,
}

/// Flash block.
#[derive(Debug, Deserialize)]
pub struct Block {
    pub header: Header,
    pub data: Entry,
}

/// Any entry - should always be either a leaf or a branch (more entries).
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Leaf(LeafEntry),
    Branch(IndexMap<String, Entry>),
}

impl Block {
    pub fn build_bytestream(
        &self,
        data_source: Option<&dyn DataSource>,
        settings: &Settings,
        strict: bool,
    ) -> Result<(Vec<u8>, u32), LayoutError> {
        let mut state = BuildState {
            buffer: Vec::with_capacity((self.header.length as usize).min(64 * 1024)),
            offset: 0,
            padding_count: 0,
        };
        let config = BuildConfig {
            endianness: &settings.endianness,
            padding: self.header.padding,
            strict,
        };

        Self::build_bytestream_inner(&self.data, data_source, &mut state, &config)?;

        // Resolve CRC config and check if keyword location (needs 4-byte alignment)
        let resolved: CrcConfig = self
            .header
            .crc
            .as_ref()
            .map(|hc| hc.resolve(settings.crc.as_ref()))
            .unwrap_or_else(|| settings.crc.clone().unwrap_or_default());

        if let Some(CrcLocation::Keyword(_)) = &resolved.location {
            while !state.offset.is_multiple_of(4) {
                state.buffer.push(config.padding);
                state.offset += 1;
                state.padding_count += 1;
            }
        }

        Ok((state.buffer, state.padding_count))
    }

    fn build_bytestream_inner(
        table: &Entry,
        data_source: Option<&dyn DataSource>,
        state: &mut BuildState,
        config: &BuildConfig,
    ) -> Result<(), LayoutError> {
        match table {
            Entry::Leaf(leaf) => {
                let alignment = leaf.get_alignment();
                while !state.offset.is_multiple_of(alignment) {
                    state.buffer.push(config.padding);
                    state.offset += 1;
                    state.padding_count += 1;
                }

                let bytes = leaf.emit_bytes(data_source, config)?;
                state.offset += bytes.len();
                state.buffer.extend(bytes);
            }
            Entry::Branch(branch) => {
                for (field_name, v) in branch.iter() {
                    Self::build_bytestream_inner(v, data_source, state, config).map_err(|e| {
                        LayoutError::InField {
                            field: field_name.clone(),
                            source: Box::new(e),
                        }
                    })?;
                }
            }
        }
        Ok(())
    }
}
