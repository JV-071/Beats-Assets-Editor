pub mod commands;
pub mod dat_reader;
pub mod mapper;
pub mod sheet_compiler;
pub mod spr_reader;
pub mod types;
pub mod versions;

pub use commands::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::protobuf::{Appearance, Appearances};
    use crate::features::sprites::parsers::SpriteLoader;
    use prost::Message;

    #[test]
    fn test_versions_definitions() {
        assert!(!versions::SUPPORTED_VERSIONS.is_empty());
        let v860 = versions::get_version_by_id(860).expect("Tibia 8.60 must exist");
        assert_eq!(v860.structure, 5);
        let v1098 = versions::get_version_by_id(1098).expect("Tibia 10.98 must exist");
        assert_eq!(v1098.structure, 6);
        assert!(v1098.default_extended);
    }

    #[test]
    fn test_sheet_compiler_and_loader_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("test_converter_rt_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let mut sprites = Vec::new();
        for id in 1..=10 {
            let mut rgba = vec![0u8; spr_reader::SPRITE_RGBA_BYTES];
            for i in 0..spr_reader::SPRITE_PIXELS {
                rgba[i * 4] = (id * 20) as u8;
                rgba[i * 4 + 1] = 100;
                rgba[i * 4 + 2] = 200;
                rgba[i * 4 + 3] = 255;
            }
            sprites.push(spr_reader::DecodedSprite { id, rgba });
        }

        let (entries, sheet_count) = sheet_compiler::compile_sprites_to_sheets(&sprites, &temp_dir).unwrap();
        assert_eq!(sheet_count, 1);
        assert_eq!(entries.len(), 1);

        let catalog_path = temp_dir.join("catalog-content.json");
        assert!(catalog_path.exists());

        let loader = SpriteLoader::new(catalog_path.to_str().unwrap(), temp_dir.to_str().unwrap()).unwrap();
        let s1 = loader.get_sprite(1).unwrap();
        assert_eq!(s1.width, 32);
        assert_eq!(s1.height, 32);
        assert_eq!(s1.data[0], 20);
        assert_eq!(s1.data[1], 100);
        assert_eq!(s1.data[2], 200);
        assert_eq!(s1.data[3], 255);

        let mut appearances = Appearances::default();
        appearances.object.push(Appearance {
            id: Some(100),
            name: Some(b"Test Sword".to_vec()),
            ..Default::default()
        });

        let dat_path = sheet_compiler::write_appearances_dat(&appearances, &temp_dir).unwrap();
        assert!(dat_path.exists());

        let dat_bytes = std::fs::read(&dat_path).unwrap();
        let decoded = Appearances::decode(dat_bytes.as_slice()).unwrap();
        assert_eq!(decoded.object.len(), 1);
        assert_eq!(decoded.object[0].id, Some(100));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
