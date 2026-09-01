use serde::{Deserialize, Serialize};

/// Options passed from the frontend for legacy DAT/SPR conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyConvertOptions {
    pub dat_path: String,
    pub spr_path: String,
    pub output_dir: String,
    /// Client version ID (e.g. 860, 1098, 0 for auto-detect)
    pub version_id: Option<u32>,
    /// Whether the SPR has extended sprite count (u32 length/offsets)
    pub extended_sprites: bool,
    /// Whether the SPR has RGBA transparency (OTClient 4 channels)
    pub transparency: bool,
    /// Whether the DAT supports frame groups (outfits idle/moving)
    pub frame_groups: bool,
    /// Whether improved animations with min/max durations are present
    pub improved_animations: bool,
    /// Whether to generate an .aec bundle file in the output directory
    pub export_aec: bool,
    /// Optional project title/name for AEC metadata
    pub project_name: Option<String>,
}

/// Metadata and detected client version info from DAT and SPR files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyDetectedInfo {
    pub dat_signature: u32,
    pub spr_signature: u32,
    pub detected_version_id: u32,
    pub detected_version_name: String,
    pub object_count: u32,
    pub outfit_count: u32,
    pub effect_count: u32,
    pub missile_count: u32,
    pub total_things: u32,
    pub sprite_count: u32,
    pub is_extended: bool,
    pub suggested_transparency: bool,
    pub suggested_frame_groups: bool,
    pub suggested_improved_animations: bool,
}

/// Information about a supported client version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedLegacyVersion {
    pub id: u32,
    pub name: String,
    pub dat_signature: u32,
    pub spr_signature: u32,
    pub structure: u8,
    pub default_extended: bool,
    pub default_frame_groups: bool,
    pub default_improved_animations: bool,
}

/// Result of the conversion process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub success: bool,
    pub output_dir: String,
    pub appearances_path: String,
    pub catalog_path: String,
    pub aec_path: Option<String>,
    pub object_count: u32,
    pub outfit_count: u32,
    pub effect_count: u32,
    pub missile_count: u32,
    pub sprites_converted: u32,
    pub sheets_created: usize,
    pub elapsed_ms: u64,
}
