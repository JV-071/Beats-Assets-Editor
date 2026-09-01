use super::dat_reader::{inspect_dat_header, LegacyDatReader};
use super::mapper::map_legacy_thing_to_appearance;
use super::sheet_compiler::{compile_sprites_to_sheets_streaming, write_aec_bundle_from_reader, write_appearances_dat};
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
            v.name.to_string(),
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
fn write_log_file(output_dir: &Path, logs: &[String]) -> String {
    let log_content = logs.join("\n");
    
    // 1. Tenta salvar na pasta de saída
    let out_file = output_dir.join("conversion_log.txt");
    let _ = std::fs::create_dir_all(output_dir);
    if let Ok(_) = std::fs::write(&out_file, &log_content) {
        return out_file.to_string_lossy().to_string();
    }

    // 2. Fallback para diretório temporário do sistema
    let temp_dir = std::env::temp_dir().join("CanaryStudio");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_file = temp_dir.join("conversion_log.txt");
    let _ = std::fs::write(&temp_file, &log_content);
    temp_file.to_string_lossy().to_string()
}

/// Converts legacy DAT and SPR files directly to modern assets (appearances.dat, catalog-content.json, LZMA sheets, and optional AEC)
#[tauri::command]
pub async fn convert_legacy_to_assets(options: LegacyConvertOptions) -> Result<ConversionResult, String> {
    // Wrap entire conversion in catch_unwind to prevent panics from crashing the app
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        convert_legacy_to_assets_inner(options)
    }));

    match result {
        Ok(inner_result) => inner_result,
        Err(panic_info) => {
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic (no message available)".to_string()
            };

            let crash_log = format!(
                "[CRASH] O conversor sofreu um panic inesperado:\n{}\n\nIsso geralmente indica dados corrompidos no arquivo .spr ou .dat.\nTente com opções diferentes (ex: Extended Sprites, Transparência RGBA).",
                panic_msg
            );
            log::error!("{}", crash_log);

            // Emergency crash log to disk
            if let Ok(temp_dir) = std::env::var("TEMP") {
                let crash_path = Path::new(&temp_dir).join("CanaryStudio").join("crash_log.txt");
                let _ = std::fs::create_dir_all(crash_path.parent().unwrap());
                let _ = std::fs::write(&crash_path, &crash_log);
            }

            Err(crash_log)
        }
    }
}

fn convert_legacy_to_assets_inner(options: LegacyConvertOptions) -> Result<ConversionResult, String> {
    let start_time = Instant::now();
    let mut logs: Vec<String> = Vec::new();
    let output_path = Path::new(&options.output_dir);

    macro_rules! log_msg {
        ($($arg:tt)*) => {{
            let msg = format!("[{:.3}s] {}", start_time.elapsed().as_secs_f32(), format!($($arg)*));
            log::info!("{}", msg);
            logs.push(msg);
        }};
    }

    macro_rules! log_err {
        ($($arg:tt)*) => {{
            let msg = format!("[{:.3}s] [ERRO] {}", start_time.elapsed().as_secs_f32(), format!($($arg)*));
            log::error!("{}", msg);
            logs.push(msg);
        }};
    }

    log_msg!("=== INÍCIO DA CONVERSÃO DE ARQUIVOS LEGADOS ===");
    log_msg!("Arquivo DAT: {:?}", options.dat_path);
    log_msg!("Arquivo SPR: {:?}", options.spr_path);
    log_msg!("Pasta de Destino: {:?}", options.output_dir);
    log_msg!("Opções: Extended={}, Transparência={}, FrameGroups={}, ImprovedAnimations={}, ExportAEC={}",
        options.extended_sprites, options.transparency, options.frame_groups, options.improved_animations, options.export_aec
    );

    // Garante a criação da pasta de destino desde o início
    if let Err(e) = std::fs::create_dir_all(output_path) {
        let err_str = format!("Não foi possível criar a pasta de destino {:?}: {}", options.output_dir, e);
        log_err!("{}", err_str);
        let log_file = write_log_file(output_path, &logs);
        return Err(format!("{}\nLog gravado em: {}", err_str, log_file));
    }

    // 1. Resolve version parameters
    log_msg!("Passo 1/6: Resolvendo versão e estrutura...");
    let version_id = options.version_id.unwrap_or(0);
    let (structure, is_extended, has_frame_groups, has_improved_animations) = if version_id > 0 {
        if let Some(v) = get_version_by_id(version_id) {
            log_msg!("Versão selecionada: {} (Structure {})", v.name, v.structure);
            (
                v.structure,
                options.extended_sprites || v.default_extended,
                options.frame_groups || v.default_frame_groups,
                options.improved_animations || v.default_improved_animations,
            )
        } else {
            log_msg!("Versão manual informada ID {} (Structure {})", version_id, get_structure_for_version(version_id));
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
            log_msg!("Versão auto-detectada com sucesso: {} (DAT: 0x{:X}, SPR: 0x{:X}, Structure: {})", v.name, dat_sig, spr_sig, v.structure);
            (
                v.structure,
                options.extended_sprites || ext || v.default_extended,
                options.frame_groups || v.default_frame_groups,
                options.improved_animations || v.default_improved_animations,
            )
        } else {
            log_msg!("Assinatura não cadastrada na tabela de versões (DAT: 0x{:X}, SPR: 0x{:X}). Usando estrutura padrão.", dat_sig, spr_sig);
            (
                if options.extended_sprites || ext { 6 } else { 5 },
                options.extended_sprites || ext,
                options.frame_groups,
                options.improved_animations,
            )
        }
    };

    // 2. Read SPR header and offsets (Zero-RAM Spikes: Sprites são lidas sob demanda em streaming)
    log_msg!("Passo 2/6: Lendo cabeçalho e tabela de offsets do SPR (is_extended={}, transparency={})...", is_extended, options.transparency);
    let spr_reader = match LegacySprReader::open(&options.spr_path, is_extended, options.transparency) {
        Ok(reader) => {
            log_msg!("SPR aberto com sucesso. Total de sprites no cabeçalho: {}", reader.sprite_count);
            reader
        }
        Err(e) => {
            let err_str = format!("Erro ao abrir arquivo SPR ({:?}): {}. Dica: Verifique se o arquivo não está corrompido ou se a opção 'Extended Sprites' precisa ser ativada/desativada.", options.spr_path, e);
            log_err!("{}", err_str);
            let log_file = write_log_file(output_path, &logs);
            return Err(format!("{}\nLog gravado em: {}", err_str, log_file));
        }
    };

    // 3. Read DAT file
    log_msg!("Passo 3/6: Lendo e interpretando arquivo DAT (structure={}, extended={}, frame_groups={}, improved_animations={})...",
        structure, is_extended, has_frame_groups, has_improved_animations);
    let dat_reader = match LegacyDatReader::open(
        &options.dat_path,
        structure,
        is_extended,
        has_frame_groups,
        has_improved_animations,
    ) {
        Ok(reader) => {
            log_msg!("DAT lido com sucesso (Structure {}, FrameGroups: {}, ImprovedAnimations: {}): {} Objects (itens), {} Outfits, {} Effects, {} Missiles",
                reader.structure, reader.has_frame_groups, reader.has_improved_animations,
                reader.object_count, reader.outfit_count, reader.effect_count, reader.missile_count);
            reader
        }
        Err(e) => {
            let err_str = format!("Erro ao ler arquivo DAT ({:?}): {}. Dica: Selecione a versão correta do cliente ou ajuste as opções de Frame Groups / Animações Avançadas.", options.dat_path, e);
            log_err!("{}", err_str);
            let log_file = write_log_file(output_path, &logs);
            return Err(format!("{}\nLog gravado em: {}", err_str, log_file));
        }
    };

    // 4. Map Things to Protobuf Appearances
    log_msg!("Passo 4/6: Mapeando categorias e atributos para o formato Protobuf moderno...");
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
    log_msg!("Mapeamento concluído: Total de {} aparências prontas.",
        appearances.object.len() + appearances.outfit.len() + appearances.effect.len() + appearances.missile.len()
    );

    // 5. Write appearances.dat
    log_msg!("Passo 5/6: Serializando e gravando appearances.dat...");
    let appearances_file = match write_appearances_dat(&appearances, output_path) {
        Ok(path) => {
            log_msg!("appearances.dat gravado com sucesso em {:?}", path);
            path
        }
        Err(e) => {
            let err_str = format!("Erro ao gravar appearances.dat: {}", e);
            log_err!("{}", err_str);
            let log_file = write_log_file(output_path, &logs);
            return Err(format!("{}\nLog gravado em: {}", err_str, log_file));
        }
    };

    // 6. Compile Sprites to Sheets in Streaming Mode (Consumo de RAM constante < 50MB)
    log_msg!("Passo 6/6: Compilando spritesheets LZMA (.cwm) em streaming contínuo...");
    let total_sprites_count = spr_reader.sprite_count;
    let (_, sheets_created) = match compile_sprites_to_sheets_streaming(
        &spr_reader,
        output_path,
        Some(|current_sheet: usize, total_sheets: usize| {
            if current_sheet % 100 == 0 || current_sheet == total_sheets {
                log::info!("Progresso: Folha {} de {} gerada...", current_sheet, total_sheets);
            }
        }),
    ) {
        Ok(res) => {
            log_msg!("Spritesheets geradas com sucesso: {} folhas criadas em {:?}", res.1, output_path);
            res
        }
        Err(e) => {
            let err_str = format!("Erro ao compilar folhas de sprites LZMA: {}", e);
            log_err!("{}", err_str);
            let log_file = write_log_file(output_path, &logs);
            return Err(format!("{}\nLog gravado em: {}", err_str, log_file));
        }
    };

    // 7. Optional AEC Export (Streaming)
    let mut aec_file_path = None;
    if options.export_aec {
        log_msg!("Exportando pacote opcional .aec...");
        let proj_name = options
            .project_name
            .as_deref()
            .unwrap_or("converted_legacy_assets");
        match write_aec_bundle_from_reader(&appearances, &spr_reader, output_path, proj_name) {
            Ok(aec_res) => {
                let aec_str = aec_res.to_string_lossy().to_string();
                log_msg!("Pacote .aec gravado com sucesso: {:?}", aec_str);
                aec_file_path = Some(aec_str);
            }
            Err(e) => {
                log_err!("Aviso: Falha ao exportar bundle AEC (os assets principais foram gerados): {}", e);
            }
        }
    }

    let elapsed = start_time.elapsed().as_millis() as u64;
    log_msg!("=== CONVERSÃO CONCLUÍDA COM SUCESSO EM {} ms ===", elapsed);

    let log_file = write_log_file(output_path, &logs);
    log_msg!("Arquivo de log final gravado em: {:?}", log_file);

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
        sprites_converted: total_sprites_count,
        sheets_created,
        elapsed_ms: elapsed,
        log_path: log_file,
        logs,
    })
}
