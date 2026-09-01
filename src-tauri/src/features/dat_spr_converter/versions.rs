use super::types::SupportedLegacyVersion;

pub const SUPPORTED_VERSIONS: &[SupportedLegacyVersion] = &[
    SupportedLegacyVersion {
        id: 710,
        name: "Tibia 7.10",
        dat_signature: 0x3DFF4B2A,
        spr_signature: 0x3DFF4AEB,
        structure: 1,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 730,
        name: "Tibia 7.30",
        dat_signature: 0x411A6233,
        spr_signature: 0x411A6279,
        structure: 1,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 740,
        name: "Tibia 7.40",
        dat_signature: 0x41BF619C,
        spr_signature: 0x41B9EA86,
        structure: 2,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 750,
        name: "Tibia 7.50",
        dat_signature: 0x42F81973,
        spr_signature: 0x42F81949,
        structure: 2,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 755,
        name: "Tibia 7.55",
        dat_signature: 0x437B2B8F,
        spr_signature: 0x434F9CDE,
        structure: 3,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 760,
        name: "Tibia 7.60 / 7.70",
        dat_signature: 0x439D5A33,
        spr_signature: 0x439852BE,
        structure: 3,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 780,
        name: "Tibia 7.80",
        dat_signature: 0x44CE4743,
        spr_signature: 0x44CE4206,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 790,
        name: "Tibia 7.90",
        dat_signature: 0x457D854E,
        spr_signature: 0x457957C8,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 792,
        name: "Tibia 7.92",
        dat_signature: 0x459E7B73,
        spr_signature: 0x45880FE8,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 800,
        name: "Tibia 8.00",
        dat_signature: 0x467FD7E6,
        spr_signature: 0x467F9E74,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 810,
        name: "Tibia 8.10",
        dat_signature: 0x475D3747,
        spr_signature: 0x475D0B01,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 820,
        name: "Tibia 8.20",
        dat_signature: 0x486905AA,
        spr_signature: 0x4868ECC9,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 830,
        name: "Tibia 8.30",
        dat_signature: 0x48DA1FB6,
        spr_signature: 0x48C8E712,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 840,
        name: "Tibia 8.40",
        dat_signature: 0x493D607A,
        spr_signature: 0x493D4E7C,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 854,
        name: "Tibia 8.54",
        dat_signature: 0x4B1E2CAA,
        spr_signature: 0x4B1E2C87,
        structure: 4,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 860,
        name: "Tibia 8.60",
        dat_signature: 0x4C28B721,
        spr_signature: 0x4C220594,
        structure: 5,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 870,
        name: "Tibia 8.70",
        dat_signature: 0x4CFE22C5,
        spr_signature: 0x4CFD078A,
        structure: 5,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 910,
        name: "Tibia 9.10",
        dat_signature: 0x4E12DAFF,
        spr_signature: 0x4E12DB27,
        structure: 5,
        default_extended: false,
        default_frame_groups: false,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 960,
        name: "Tibia 9.60",
        dat_signature: 0x4FFA74CC,
        spr_signature: 0x4FFA74F9,
        structure: 5,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 986,
        name: "Tibia 9.86",
        dat_signature: 0x5170E904,
        spr_signature: 0x5170E96F,
        structure: 5,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: false,
    },
    SupportedLegacyVersion {
        id: 1010,
        name: "Tibia 10.10",
        dat_signature: 0x51E3F8C3,
        spr_signature: 0x51E3F8E9,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
    SupportedLegacyVersion {
        id: 1035,
        name: "Tibia 10.35",
        dat_signature: 0x52FDFC2C,
        spr_signature: 0x52FDFC54,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
    SupportedLegacyVersion {
        id: 1070,
        name: "Tibia 10.70",
        dat_signature: 0x5481BB97,
        spr_signature: 0x5481BC06,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
    SupportedLegacyVersion {
        id: 1098,
        name: "Tibia 10.98",
        dat_signature: 0x42A3,
        spr_signature: 0x57BBD603,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
    SupportedLegacyVersion {
        id: 1099,
        name: "Tibia 10.99",
        dat_signature: 0x4347,
        spr_signature: 0x57FF106B,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
    SupportedLegacyVersion {
        id: 1310,
        name: "Tibia 13.10",
        dat_signature: 0x4A10,
        spr_signature: 0x59E48E02,
        structure: 6,
        default_extended: true,
        default_frame_groups: true,
        default_improved_animations: true,
    },
];

/// Finds a version definition by ID
pub fn get_version_by_id(id: u32) -> Option<&'static SupportedLegacyVersion> {
    SUPPORTED_VERSIONS.iter().find(|v| v.id == id)
}

/// Finds a version definition matching DAT and SPR signatures
pub fn detect_version_from_signatures(dat_sig: u32, spr_sig: u32) -> Option<&'static SupportedLegacyVersion> {
    // 1. Try exact match on both
    if let Some(v) = SUPPORTED_VERSIONS.iter().find(|v| v.dat_signature == dat_sig && v.spr_signature == spr_sig) {
        return Some(v);
    }
    // 2. Try match on DAT signature
    if let Some(v) = SUPPORTED_VERSIONS.iter().find(|v| v.dat_signature == dat_sig) {
        return Some(v);
    }
    // 3. Try match on SPR signature
    if let Some(v) = SUPPORTED_VERSIONS.iter().find(|v| v.spr_signature == spr_sig) {
        return Some(v);
    }
    None
}

/// Determines the metadata structure type (1 to 6) from a version ID
pub fn get_structure_for_version(version_id: u32) -> u8 {
    if let Some(v) = get_version_by_id(version_id) {
        return v.structure;
    }
    if version_id <= 730 {
        1
    } else if version_id <= 750 {
        2
    } else if version_id <= 772 {
        3
    } else if version_id <= 854 {
        4
    } else if version_id <= 986 {
        5
    } else {
        6
    }
}
