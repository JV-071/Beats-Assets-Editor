use super::dat_reader::{inspect_dat_header, LegacyDatReader};
use super::mapper::map_legacy_thing_to_appearance;
use super::sheet_compiler::{compile_sprites_to_sheets, write_aec_bundle, write_appearances_dat};
use super::spr_reader::{inspect_spr_header, LegacySprReader};
use super::types::{ConversionResult, LegacyConvertOptions, LegacyDetectedInfo, SupportedLegacyVersion};
use super::versions::{detect_version_from_signatures, get_structure_for_version, get_version_by_id, SUPPORTED_VERSIONS};
use crate::core::protobuf::Appearances;
use std::path::Path;
use std::time::Instant;

/// Inspects legacy DAT and SPR files and attempts auto-detection of client version
#[tauri::command]
pub async fn detect_legacy_files(dat_path: String, spr_path: String) -> Result<LegacyDetectedInfo, String> {
    let (dat_sig, object_count, outfit_count, effect_count, missile_count) =
        inspect_dat_header(&dat_path).map_err(|e| format!("Failed to inspect DAT header: {}", e))?;

    let (spr_sig, sprite_count, is_extended) =
        inspect_spr_header(&spr_path).map_err(|e| format!("Failed to inspect SPR header: {}", e))?;

    let total_things = object_count
        .saturating_sub(99)
        .saturating_add(outfit_count)
        .saturating_add(effect_count)
        .saturating_add(missile_count);

    let detected_version = detect_version_from_signatures(dat_sig, spr_sig);
    let (ver_id, ver_name, suggested_extended, suggested_fg, suggested_ia) = match detected_version {
        Some(v) => (
            v.id,
            v.name.clone(),
            v.default_extended,
            v.default_frame_groups,
            v.default_improved_animations,
        ),
        None => {
            // Heuristic fallback
            if is_extended || sprite_count > 65535 {
                (1098, "Tibia 10.98 (Custom / Detected)".to_string(), true, true, true)
            } else {
                (860, "Tibia 8.60 (Custom / Detected)".to_string(), false, false, false)
            }
        }
    };

    Ok(LegacyDetectedInfo {
        dat_signature: dat_sig,
        spr_signature: spr_sig,
        detected_version_id: ver_id,
        detected_version_name: ver_name,
        object_count,
        outfit_count,
        effect_count,
        missile_count,
        total_things,
        sprite_count,
        is_extended: is_extended || suggested_extended,
        suggested_transparency: false, // Default false unless selected
        suggested_frame_groups: suggested_fg,
        suggested_improved_animations: suggested_ia,
    })
}

/// Returns list of all supported legacy client versions
#[tauri::command]
pub fn get_supported_legacy_versions() -> Vec<SupportedLegacyVersion> {
    SUPPORTED_VERSIONS.to_vec()
}

/// Converts legacy DAT and SPR files directly to modern assets (appearances.dat, catalog-content.json, LZMA sheets, and optional AEC)
#[tauri::command]
pub async fn convert_legacy_to_assets(options: LegacyConvertOptions) -> Result<ConversionResult, String> {
    let start_time = Instant::now();

    log::info!("Starting legacy conversion for DAT: {:?} and SPR: {:?}", options.dat_path, options.spr_path);

    // 1. Resolve version parameters
    let version_id = options.version_id.unwrap_or(0);
    let (structure, is_extended, has_frame_groups, has_improved_animations) = if version_id > 0 {
        if let Some(v) = get_version_by_id(version_id) {
            (
                v.structure,
                options.extended_sprites || v.default_extended,
                options.frame_groups || v.default_frame_groups,
                options.improved_animations || v.default_improved_animations,
            )
        } else {
            (
                get_structure_for_version(version_id),
                options.extended_sprites,
                options.frame_groups,
                options.improved_animations,
            )
        }
    } else {
        // Auto-detect structure
        let (dat_sig, _) = inspect_dat_header(&options.dat_path)
            .map(|(s, ..)| (s, ()))
            .unwrap_or((0, ()));
        let (spr_sig, _, ext) = inspect_spr_header(&options.spr_path)
            .unwrap_or((0, 0, false));
        let det = detect_version_from_signatures(dat_sig, spr_sig);
        if let Some(v) = det {
            (
                v.structure,
                options.extended_sprites || ext || v.default_extended,
                options.frame_groups || v.default_frame_groups,
                options.improved_animations || v.default_improved_animations,
            )
        } else {
            (
                if options.extended_sprites || ext { 6 } else { 5 },
                options.extended_sprites || ext,
                options.frame_groups,
                options.improved_animations,
            )
        }
    };

    // 2. Read and decode SPR file in parallel
    log::info!(
        "Reading SPR file (extended={}, transparency={})...",
        is_extended,
        options.transparency
    );
    let spr_reader = LegacySprReader::open(&options.spr_path, is_extended, options.transparency)
        .map_err(|e| format!("Error opening SPR file: {}", e))?;

    let decoded_sprites = spr_reader
        .decode_all_sprites()
        .map_err(|e| format!("Error decoding SPR sprites: {}", e))?;
    log::info!("Successfully decoded {} sprites", decoded_sprites.len());

    // 3. Read DAT file
    log::info!(
        "Reading DAT file (structure={}, extended={}, frame_groups={}, improved_animations={})...",
        structure,
        is_extended,
        has_frame_groups,
        has_improved_animations
    );
    let dat_reader = LegacyDatReader::open(
        &options.dat_path,
        structure,
        is_extended,
        has_frame_groups,
        has_improved_animations,
    )
    .map_err(|e| format!("Error reading DAT file: {}", e))?;

    // 4. Map Things to Protobuf Appearances
    log::info!("Mapping things to Protobuf Appearances...");
    let mut appearances = Appearances::default();

    for obj in &dat_reader.objects {
        appearances.object.push(map_legacy_thing_to_appearance(obj));
    }
    for outfit in &dat_reader.outfits {
        appearances.outfit.push(map_legacy_thing_to_appearance(outfit));
    }
    for effect in &dat_reader.effects {
        appearances.effect.push(map_legacy_thing_to_appearance(effect));
    }
    for missile in &dat_reader.missiles {
        appearances.missile.push(map_legacy_thing_to_appearance(missile));
    }

    // 5. Compile Sprites to Sheets & write catalog-content.json
    log::info!("Compiling sprites to LZMA sprite sheets and generating catalog...");
    let output_path = Path::new(&options.output_dir);
    let (_, sheets_created) = compile_sprites_to_sheets(&decoded_sprites, output_path)
        .map_err(|e| format!("Error compiling spritesheets: {}", e))?;

    // 6. Write appearances.dat
    log::info!("Writing appearances.dat...");
    let appearances_file = write_appearances_dat(&appearances, output_path)
        .map_err(|e| format!("Error writing appearances.dat: {}", e))?;

    // 7. Optional AEC Export
    let mut aec_file_path = None;
    if options.export_aec {
        log::info!("Exporting .aec bundle...");
        let proj_name = options
            .project_name
            .as_deref()
            .unwrap_or("converted_legacy_assets");
        let aec_res = write_aec_bundle(&appearances, &decoded_sprites, output_path, proj_name)
            .map_err(|e| format!("Error exporting AEC bundle: {}", e))?;
        aec_file_path = Some(aec_res.to_string_lossy().to_string());
    }

    let elapsed = start_time.elapsed().as_millis() as u64;
    log::info!("Conversion finished successfully in {} ms", elapsed);

    Ok(ConversionResult {
        success: true,
        output_dir: options.output_dir.clone(),
        appearances_path: appearances_file.to_string_lossy().to_string(),
        catalog_path: output_path.join("catalog-content.json").to_string_lossy().to_string(),
        aec_path: aec_file_path,
        object_count: dat_reader.object_count,
        outfit_count: dat_reader.outfit_count,
        effect_count: dat_reader.effect_count,
        missile_count: dat_reader.missile_count,
        sprites_converted: decoded_sprites.len() as u32,
        sheets_created,
        elapsed_ms: elapsed,
    })
}
