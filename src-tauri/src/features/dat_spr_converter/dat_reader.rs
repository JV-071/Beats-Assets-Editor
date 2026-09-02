use anyhow::{anyhow, Context, Result};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegacyCategory {
    #[default]
    Item,
    Outfit,
    Effect,
    Missile,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyFrameDuration {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyFrameGroup {
    pub group_type: u8,
    pub width: u8,
    pub height: u8,
    pub exact_size: u8,
    pub layers: u8,
    pub pattern_x: u8,
    pub pattern_y: u8,
    pub pattern_z: u8,
    pub frames: u8,
    pub is_animation: bool,
    pub animation_mode: u8,
    pub loop_count: i32,
    pub start_frame: i8,
    pub frame_durations: Vec<LegacyFrameDuration>,
    pub sprite_ids: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyLight {
    pub level: u16,
    pub color: u16,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyShift {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyMarket {
    pub category: u16,
    pub trade_as: u16,
    pub show_as: u16,
    pub name: String,
    pub restrict_profession: u16,
    pub restrict_level: u16,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyThingType {
    pub id: u32,
    pub category: LegacyCategory,
    // Flags
    pub is_ground: bool,
    pub ground_speed: u16,
    pub is_ground_border: bool,
    pub is_on_bottom: bool,
    pub is_on_top: bool,
    pub is_container: bool,
    pub is_stackable: bool,
    pub is_force_use: bool,
    pub is_multi_use: bool,
    pub is_usable: bool,
    pub is_writable: bool,
    pub max_read_write_chars: u16,
    pub is_writable_once: bool,
    pub max_read_chars: u16,
    pub is_fluid_container: bool,
    pub is_fluid: bool,
    pub is_unpassable: bool,
    pub is_unmoveable: bool,
    pub is_block_missile: bool,
    pub is_block_pathfind: bool,
    pub is_no_move_animation: bool,
    pub is_pickupable: bool,
    pub is_hangable: bool,
    pub is_vertical: bool,
    pub is_horizontal: bool,
    pub is_rotatable: bool,
    pub has_light: bool,
    pub light: LegacyLight,
    pub dont_hide: bool,
    pub is_translucent: bool,
    pub has_offset: bool,
    pub offset: LegacyShift,
    pub has_elevation: bool,
    pub elevation: u16,
    pub is_lying_object: bool,
    pub is_animate_always: bool,
    pub is_mini_map: bool,
    pub mini_map_color: u16,
    pub is_lens_help: bool,
    pub lens_help: u16,
    pub is_full_ground: bool,
    pub is_ignore_look: bool,
    pub is_cloth: bool,
    pub cloth_slot: u16,
    pub is_market_item: bool,
    pub market: LegacyMarket,
    pub has_default_action: bool,
    pub default_action: u16,
    pub is_wrappable: bool,
    pub is_unwrappable: bool,
    pub is_top_effect: bool,
    // Frame groups
    pub frame_groups: Vec<LegacyFrameGroup>,
}

pub struct LegacyDatReader {
    pub signature: u32,
    pub object_count: u32,
    pub outfit_count: u32,
    pub effect_count: u32,
    pub missile_count: u32,
    pub structure: u8,
    pub is_extended: bool,
    pub has_frame_groups: bool,
    pub has_improved_animations: bool,
    pub objects: Vec<LegacyThingType>,
    pub outfits: Vec<LegacyThingType>,
    pub effects: Vec<LegacyThingType>,
    pub missiles: Vec<LegacyThingType>,
}

impl LegacyDatReader {
    pub fn open<P: AsRef<Path>>(
        path: P,
        structure: u8,
        is_extended: bool,
        has_frame_groups: bool,
        has_improved_animations: bool,
    ) -> Result<Self> {
        let path = path.as_ref();
        let file_bytes = std::fs::read(path).context(format!("Failed to read DAT file: {:?}", path))?;

        if file_bytes.len() < 12 {
            return Err(anyhow!("DAT file too small (less than 12 bytes)"));
        }

        // Tenta primeiro a configuração solicitada
        let candidates = vec![
            (structure, has_frame_groups, has_improved_animations),
            (structure, has_frame_groups, false),
            (structure, false, false),
            (structure, false, true),
            // Fallbacks de estrutura alternativa (se 6, tenta 5; se 5, tenta 6)
            (if structure == 6 { 5 } else { 6 }, has_frame_groups, false),
            (if structure == 6 { 5 } else { 6 }, false, false),
        ];

        let mut first_error: Option<anyhow::Error> = None;
        for (struct_candidate, fg_candidate, ia_candidate) in candidates {
            match Self::try_parse_bytes(&file_bytes, struct_candidate, is_extended, fg_candidate, ia_candidate) {
                Ok(reader) => {
                    log::info!(
                        "DAT interpretado com sucesso (structure={}, extended={}, frame_groups={}, improved_animations={})",
                        struct_candidate, is_extended, fg_candidate, ia_candidate
                    );
                    return Ok(reader);
                }
                Err(e) => {
                    log::debug!("Candidato (structure={}, fg={}, ia={}) falhou: {}", struct_candidate, fg_candidate, ia_candidate, e);
                    if first_error.is_none() {
                        first_error = Some(anyhow!("Tentativa principal (structure={}, fg={}, ia={}) falhou: {}", struct_candidate, fg_candidate, ia_candidate, e));
                    }
                }
            }
        }

        Err(first_error.unwrap_or_else(|| anyhow!("Falha ao ler DAT em todas as combinações de estrutura")))
    }


    fn try_parse_bytes(
        file_bytes: &[u8],
        structure: u8,
        is_extended: bool,
        has_frame_groups: bool,
        has_improved_animations: bool,
    ) -> Result<Self> {
        let mut cursor = Cursor::new(file_bytes);
        let signature = cursor.read_u32::<LittleEndian>().context("Failed to read signature")?;
        let object_count = cursor.read_u16::<LittleEndian>().context("Failed to read object count")? as u32;
        let outfit_count = cursor.read_u16::<LittleEndian>().context("Failed to read outfit count")? as u32;
        let effect_count = cursor.read_u16::<LittleEndian>().context("Failed to read effect count")? as u32;
        let missile_count = cursor.read_u16::<LittleEndian>().context("Failed to read missile count")? as u32;

        let mut objects = Vec::with_capacity((object_count.saturating_sub(99)) as usize);
        for id in 100..=object_count {
            let thing = read_thing(&mut cursor, id, LegacyCategory::Item, structure, is_extended, has_frame_groups, has_improved_animations)?;
            objects.push(thing);
        }

        let mut outfits = Vec::with_capacity(outfit_count as usize);
        for id in 1..=outfit_count {
            let thing = read_thing(&mut cursor, id, LegacyCategory::Outfit, structure, is_extended, has_frame_groups, has_improved_animations)?;
            outfits.push(thing);
        }

        let mut effects = Vec::with_capacity(effect_count as usize);
        for id in 1..=effect_count {
            let thing = read_thing(&mut cursor, id, LegacyCategory::Effect, structure, is_extended, has_frame_groups, has_improved_animations)?;
            effects.push(thing);
        }

        let mut missiles = Vec::with_capacity(missile_count as usize);
        for id in 1..=missile_count {
            let thing = read_thing(&mut cursor, id, LegacyCategory::Missile, structure, is_extended, has_frame_groups, has_improved_animations)?;
            missiles.push(thing);
        }

        Ok(Self {
            signature,
            object_count,
            outfit_count,
            effect_count,
            missile_count,
            structure,
            is_extended,
            has_frame_groups,
            has_improved_animations,
            objects,
            outfits,
            effects,
            missiles,
        })
    }

}

pub fn inspect_dat_header<P: AsRef<Path>>(path: P) -> Result<(u32, u32, u32, u32, u32)> {
    let path = path.as_ref();
    let mut file = File::open(path).context("Failed to open DAT file")?;

    let signature = file.read_u32::<LittleEndian>()?;
    let object_count = file.read_u16::<LittleEndian>()? as u32;
    let outfit_count = file.read_u16::<LittleEndian>()? as u32;
    let effect_count = file.read_u16::<LittleEndian>()? as u32;
    let missile_count = file.read_u16::<LittleEndian>()? as u32;

    Ok((signature, object_count, outfit_count, effect_count, missile_count))
}

fn read_thing<R: Read + Seek>(
    cursor: &mut R,
    id: u32,
    category: LegacyCategory,
    structure: u8,
    is_extended: bool,
    has_frame_groups: bool,
    has_improved_animations: bool,
) -> Result<LegacyThingType> {
    let mut thing = LegacyThingType {
        id,
        category,
        ..Default::default()
    };

    // 1. Read Properties
    read_properties(cursor, &mut thing, structure)?;

    // 2. Read Textures & FrameGroups
    read_texture_patterns(cursor, &mut thing, structure, is_extended, has_frame_groups, has_improved_animations)?;

    Ok(thing)
}

fn read_properties<R: Read>(cursor: &mut R, thing: &mut LegacyThingType, structure: u8) -> Result<()> {
    let mut attr_count = 0usize;
    loop {
        attr_count += 1;
        if attr_count > 128 {
            return Err(anyhow!("Thing {} has more than 128 attributes (possible corrupt or desynchronized dat)", thing.id));
        }

        let flag = cursor.read_u8().context("Failed to read property flag")?;
        if flag == 0xFF {
            break;
        }

        match structure {
            1 => parse_flags_v1(cursor, thing, flag)?,
            2 => parse_flags_v2(cursor, thing, flag)?,
            3 => parse_flags_v3(cursor, thing, flag)?,
            4 => parse_flags_v4(cursor, thing, flag)?,
            5 => parse_flags_v5(cursor, thing, flag)?,
            _ => parse_flags_v6(cursor, thing, flag)?,
        }
    }
    Ok(())
}

fn parse_flags_v1<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_on_bottom = true; }
        0x02 => { t.is_on_top = true; }
        0x03 => { t.is_container = true; }
        0x04 => { t.is_stackable = true; }
        0x05 => { t.is_multi_use = true; }
        0x06 => { t.is_force_use = true; }
        0x07 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x08 => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x09 => { t.is_fluid_container = true; }
        0x0A => { t.is_fluid = true; }
        0x0B => { t.is_unpassable = true; }
        0x0C => { t.is_unmoveable = true; }
        0x0D => { t.is_block_missile = true; }
        0x0E => { t.is_block_pathfind = true; }
        0x0F => { t.is_pickupable = true; }
        0x10 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x11 => { /* Floor change */ }
        0x12 => { t.is_full_ground = true; }
        0x13 => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x14 => {
            t.has_offset = true;
            t.offset.x = 8;
            t.offset.y = 8;
        }
        0x16 => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x17 => { t.is_rotatable = true; }
        0x18 => { t.is_lying_object = true; }
        0x19 => { t.is_animate_always = true; }
        0x1A => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x24 => { t.is_wrappable = true; }
        0x25 => { t.is_unwrappable = true; }
        0x26 => { t.is_top_effect = true; }
        _ => return Err(anyhow!("Unknown flag 0x{:02X} in struct 1", flag)),
    }
    Ok(())
}

fn parse_flags_v2<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_on_bottom = true; }
        0x02 => { t.is_on_top = true; }
        0x03 => { t.is_container = true; }
        0x04 => { t.is_stackable = true; }
        0x05 => { t.is_multi_use = true; }
        0x06 => { t.is_force_use = true; }
        0x07 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x08 => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x09 => { t.is_fluid_container = true; }
        0x0A => { t.is_fluid = true; }
        0x0B => { t.is_unpassable = true; }
        0x0C => { t.is_unmoveable = true; }
        0x0D => { t.is_block_missile = true; }
        0x0E => { t.is_block_pathfind = true; }
        0x0F => { t.is_pickupable = true; }
        0x10 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x11 => { /* Floor change */ }
        0x12 => { t.is_full_ground = true; }
        0x13 => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x14 => {
            t.has_offset = true;
            t.offset.x = 8;
            t.offset.y = 8;
        }
        0x16 => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x17 => { t.is_rotatable = true; }
        0x18 => { t.is_lying_object = true; }
        0x19 => { t.is_hangable = true; }
        0x1A => { t.is_vertical = true; }
        0x1B => { t.is_horizontal = true; }
        0x1C => { t.is_animate_always = true; }
        0x1D => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x24 => { t.is_wrappable = true; }
        0x25 => { t.is_unwrappable = true; }
        0x26 => { t.is_top_effect = true; }
        _ => return Err(anyhow!("Unknown flag 0x{:02X} in struct 2", flag)),
    }
    Ok(())
}

fn parse_flags_v3<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_ground_border = true; }
        0x02 => { t.is_on_bottom = true; }
        0x03 => { t.is_on_top = true; }
        0x04 => { t.is_container = true; }
        0x05 => { t.is_stackable = true; }
        0x06 => { t.is_multi_use = true; }
        0x07 => { t.is_force_use = true; }
        0x08 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x09 => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x0A => { t.is_fluid_container = true; }
        0x0B => { t.is_fluid = true; }
        0x0C => { t.is_unpassable = true; }
        0x0D => { t.is_unmoveable = true; }
        0x0E => { t.is_block_missile = true; }
        0x0F => { t.is_block_pathfind = true; }
        0x10 => { t.is_pickupable = true; }
        0x11 => { t.is_hangable = true; }
        0x12 => { t.is_vertical = true; }
        0x13 => { t.is_horizontal = true; }
        0x14 => { t.is_rotatable = true; }
        0x15 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x16 => { /* Floor change */ }
        0x17 => {
            t.has_offset = true;
            t.offset.x = cursor.read_i16::<LittleEndian>()?;
            t.offset.y = cursor.read_i16::<LittleEndian>()?;
        }
        0x18 => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x19 => { t.is_lying_object = true; }
        0x1A => { t.is_animate_always = true; }
        0x1B => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x1C => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x1D => { t.is_full_ground = true; }
        _ => return Err(anyhow!("Unknown flag 0x{:02X} in struct 3", flag)),
    }
    Ok(())
}

fn parse_flags_v4<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_ground_border = true; }
        0x02 => { t.is_on_bottom = true; }
        0x03 => { t.is_on_top = true; }
        0x04 => { t.is_container = true; }
        0x05 => { t.is_stackable = true; }
        0x06 => { t.is_force_use = true; }
        0x07 => { t.is_multi_use = true; }
        0x08 => { /* has charges */ }
        0x09 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x0A => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x0B => { t.is_fluid_container = true; }
        0x0C => { t.is_fluid = true; }
        0x0D => { t.is_unpassable = true; }
        0x0E => { t.is_unmoveable = true; }
        0x0F => { t.is_block_missile = true; }
        0x10 => { t.is_block_pathfind = true; }
        0x11 => { t.is_pickupable = true; }
        0x12 => { t.is_hangable = true; }
        0x13 => { t.is_vertical = true; }
        0x14 => { t.is_horizontal = true; }
        0x15 => { t.is_rotatable = true; }
        0x16 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x17 => { t.dont_hide = true; }
        0x18 => { /* Floor change */ }
        0x19 => {
            t.has_offset = true;
            t.offset.x = cursor.read_i16::<LittleEndian>()?;
            t.offset.y = cursor.read_i16::<LittleEndian>()?;
        }
        0x1A => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x1B => { t.is_lying_object = true; }
        0x1C => { t.is_animate_always = true; }
        0x1D => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x1E => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x1F => { t.is_full_ground = true; }
        0x20 => { t.is_ignore_look = true; }
        0x24 => { t.is_wrappable = true; }
        0x25 => { t.is_unwrappable = true; }
        0x27 => { t.is_usable = true; }
        _ => return Err(anyhow!("Unknown flag 0x{:02X} in struct 4", flag)),
    }
    Ok(())
}

fn parse_flags_v5<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_ground_border = true; }
        0x02 => { t.is_on_bottom = true; }
        0x03 => { t.is_on_top = true; }
        0x04 => { t.is_container = true; }
        0x05 => { t.is_stackable = true; }
        0x06 => { t.is_force_use = true; }
        0x07 => { t.is_multi_use = true; }
        0x08 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x09 => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x0A => { t.is_fluid_container = true; }
        0x0B => { t.is_fluid = true; }
        0x0C => { t.is_unpassable = true; }
        0x0D => { t.is_unmoveable = true; }
        0x0E => { t.is_block_missile = true; }
        0x0F => { t.is_block_pathfind = true; }
        0x10 => { t.is_pickupable = true; }
        0x11 => { t.is_hangable = true; }
        0x12 => { t.is_vertical = true; }
        0x13 => { t.is_horizontal = true; }
        0x14 => { t.is_rotatable = true; }
        0x15 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x16 => { t.dont_hide = true; }
        0x17 => { t.is_translucent = true; }
        0x18 => {
            t.has_offset = true;
            t.offset.x = cursor.read_i16::<LittleEndian>()?;
            t.offset.y = cursor.read_i16::<LittleEndian>()?;
        }
        0x19 => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x1A => { t.is_lying_object = true; }
        0x1B => { t.is_animate_always = true; }
        0x1C => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x1D => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x1E => { t.is_full_ground = true; }
        0x1F => { t.is_ignore_look = true; }
        0x20 => { t.is_cloth = true; t.cloth_slot = cursor.read_u16::<LittleEndian>()?; }
        0x21 => {
            t.is_market_item = true;
            t.market.category = cursor.read_u16::<LittleEndian>()?;
            t.market.trade_as = cursor.read_u16::<LittleEndian>()?;
            t.market.show_as = cursor.read_u16::<LittleEndian>()?;
            let name_len = cursor.read_u16::<LittleEndian>()? as usize;
            if name_len > 512 {
                return Err(anyhow!("Invalid market name length {}", name_len));
            }
            let mut name_bytes = vec![0u8; name_len];
            cursor.read_exact(&mut name_bytes)?;
            t.market.name = String::from_utf8_lossy(&name_bytes).to_string();
            t.market.restrict_profession = cursor.read_u16::<LittleEndian>()?;
            t.market.restrict_level = cursor.read_u16::<LittleEndian>()?;
        }
        0x27 => { t.is_usable = true; }
        0x28 => { let _ = cursor.read_u8(); }
        0x29 => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2A => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2B => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2C..=0x3F => { /* custom flags without data */ }
        0x64 => { let _ = cursor.read_u8(); }
        0x65 => { /* not walkable */ }
        0x66 => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x67 => { /* floor change */ }
        0x68 => { /* custom flag without data */ }
        0x69 => { /* flag extended / otclient */ }
        0x6A..=0xFD => { /* custom flags without data */ }
        0xFE => { t.is_usable = true; }
        _ => {
            log::trace!("Ignorando flag 0x{:02X} no item {}", flag, t.id);
        }
    }
    Ok(())
}

fn parse_flags_v6<R: Read>(cursor: &mut R, t: &mut LegacyThingType, flag: u8) -> Result<()> {
    match flag {
        0x00 => { t.is_ground = true; t.ground_speed = cursor.read_u16::<LittleEndian>()?; }
        0x01 => { t.is_ground_border = true; }
        0x02 => { t.is_on_bottom = true; }
        0x03 => { t.is_on_top = true; }
        0x04 => { t.is_container = true; }
        0x05 => { t.is_stackable = true; }
        0x06 => { t.is_force_use = true; }
        0x07 => { t.is_multi_use = true; }
        0x08 => { t.is_writable = true; t.max_read_write_chars = cursor.read_u16::<LittleEndian>()?; }
        0x09 => { t.is_writable_once = true; t.max_read_chars = cursor.read_u16::<LittleEndian>()?; }
        0x0A => { t.is_fluid_container = true; }
        0x0B => { t.is_fluid = true; }
        0x0C => { t.is_unpassable = true; }
        0x0D => { t.is_unmoveable = true; }
        0x0E => { t.is_block_missile = true; }
        0x0F => { t.is_block_pathfind = true; }
        0x10 => { t.is_no_move_animation = true; }
        0x11 => { t.is_pickupable = true; }
        0x12 => { t.is_hangable = true; }
        0x13 => { t.is_vertical = true; }
        0x14 => { t.is_horizontal = true; }
        0x15 => { t.is_rotatable = true; }
        0x16 => {
            t.has_light = true;
            t.light.level = cursor.read_u16::<LittleEndian>()?;
            t.light.color = cursor.read_u16::<LittleEndian>()?;
        }
        0x17 => { t.dont_hide = true; }
        0x18 => { t.is_translucent = true; }
        0x19 => {
            t.has_offset = true;
            t.offset.x = cursor.read_i16::<LittleEndian>()?;
            t.offset.y = cursor.read_i16::<LittleEndian>()?;
        }
        0x1A => { t.has_elevation = true; t.elevation = cursor.read_u16::<LittleEndian>()?; }
        0x1B => { t.is_lying_object = true; }
        0x1C => { t.is_animate_always = true; }
        0x1D => { t.is_mini_map = true; t.mini_map_color = cursor.read_u16::<LittleEndian>()?; }
        0x1E => { t.is_lens_help = true; t.lens_help = cursor.read_u16::<LittleEndian>()?; }
        0x1F => { t.is_full_ground = true; }
        0x20 => { t.is_ignore_look = true; }
        0x21 => { t.is_cloth = true; t.cloth_slot = cursor.read_u16::<LittleEndian>()?; }
        0x22 => {
            t.is_market_item = true;
            t.market.category = cursor.read_u16::<LittleEndian>()?;
            t.market.trade_as = cursor.read_u16::<LittleEndian>()?;
            t.market.show_as = cursor.read_u16::<LittleEndian>()?;
            let name_len = cursor.read_u16::<LittleEndian>()? as usize;
            if name_len > 512 {
                return Err(anyhow!("Invalid market name length {}", name_len));
            }
            let mut name_bytes = vec![0u8; name_len];
            cursor.read_exact(&mut name_bytes)?;
            t.market.name = String::from_utf8_lossy(&name_bytes).to_string();
            t.market.restrict_profession = cursor.read_u16::<LittleEndian>()?;
            t.market.restrict_level = cursor.read_u16::<LittleEndian>()?;
        }
        0x23 => { t.has_default_action = true; t.default_action = cursor.read_u16::<LittleEndian>()?; }
        0x24 => { t.is_wrappable = true; }
        0x25 => { t.is_unwrappable = true; }
        0x26 => { t.is_top_effect = true; }
        0x27 => { t.is_usable = true; }
        0x28 => { let _ = cursor.read_u8(); }
        0x29 => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2A => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2B => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x2C..=0x3F => { /* custom flags without data */ }
        0x64 => { let _ = cursor.read_u8(); }
        0x65 => { /* not walkable */ }
        0x66 => { let _ = cursor.read_u16::<LittleEndian>(); }
        0x67 => { /* floor change */ }
        0x68 => { /* custom flag without data */ }
        0x69 => { /* flag extended / otclient */ }
        0x6A..=0xFD => { /* custom flags without data */ }
        0xFE => { t.is_usable = true; }
        _ => {
            log::trace!("Ignorando flag 0x{:02X} no item {}", flag, t.id);
        }
    }
    Ok(())
}

fn read_texture_patterns<R: Read>(
    cursor: &mut R,
    thing: &mut LegacyThingType,
    structure: u8,
    is_extended: bool,
    has_frame_groups: bool,
    has_improved_animations: bool,
) -> Result<()> {
    let group_count = if has_frame_groups && thing.category == LegacyCategory::Outfit {
        let count = cursor.read_u8().context("Failed to read group count")?;
        if count == 0 { 1 } else { count }
    } else {
        1
    };

    if group_count > 4 {
        return Err(anyhow!("Invalid group count {} for thing {}", group_count, thing.id));
    }

    let use_pattern_z = structure >= 3;

    for group_idx in 0..group_count {
        let group_type = if has_frame_groups && thing.category == LegacyCategory::Outfit {
            cursor.read_u8().context("Failed to read group type")?
        } else {
            group_idx
        };

        let width = cursor.read_u8().context("Failed to read width")?;
        let height = cursor.read_u8().context("Failed to read height")?;
        let exact_size = if width > 1 || height > 1 {
            cursor.read_u8().context("Failed to read exact size")?
        } else {
            32
        };

        let layers = cursor.read_u8().context("Failed to read layers")?;
        let pattern_x = cursor.read_u8().context("Failed to read pattern_x")?;
        let pattern_y = cursor.read_u8().context("Failed to read pattern_y")?;
        let pattern_z = if use_pattern_z {
            cursor.read_u8().context("Failed to read pattern_z")?
        } else {
            1
        };
        let frames = cursor.read_u8().context("Failed to read frames")?;

        let total_sprites = (width as usize)
            .saturating_mul(height as usize)
            .saturating_mul(layers as usize)
            .saturating_mul(pattern_x as usize)
            .saturating_mul(pattern_y as usize)
            .saturating_mul(pattern_z as usize)
            .saturating_mul(frames as usize);

        if total_sprites == 0 || total_sprites > 4096 {
            return Err(anyhow!(
                "Invalid sprite count for thing {}: {}x{}, layers={}, px={}, py={}, pz={}, frames={}, total_sprites={}",
                thing.id, width, height, layers, pattern_x, pattern_y, pattern_z, frames, total_sprites
            ));
        }

        let mut is_animation = false;
        let mut animation_mode = 0u8;
        let mut loop_count = 0i32;
        let mut start_frame = 0i8;
        let mut frame_durations = Vec::new();

        if frames > 1 {
            is_animation = true;
            if has_improved_animations && thing.category == LegacyCategory::Item {
                animation_mode = cursor.read_u8().context("Failed to read animation mode")?;
                loop_count = cursor.read_i32::<LittleEndian>().context("Failed to read loop count")?;
                start_frame = cursor.read_i8().context("Failed to read start frame")?;

                for _ in 0..frames {
                    let min = cursor.read_u32::<LittleEndian>().context("Failed to read duration min")?;
                    let max = cursor.read_u32::<LittleEndian>().context("Failed to read duration max")?;
                    frame_durations.push(LegacyFrameDuration { min, max });
                }
            } else {
                let default_duration = match thing.category {
                    LegacyCategory::Item => 100,
                    LegacyCategory::Outfit => 100,
                    LegacyCategory::Effect => 75,
                    LegacyCategory::Missile => 75,
                };
                for _ in 0..frames {
                    frame_durations.push(LegacyFrameDuration {
                        min: default_duration,
                        max: default_duration,
                    });
                }
            }
        }


        let mut sprite_ids = Vec::with_capacity(total_sprites);
        for _ in 0..total_sprites {
            let sid = if is_extended {
                cursor.read_u32::<LittleEndian>().context("Failed to read sprite id u32")?
            } else {
                cursor.read_u16::<LittleEndian>().context("Failed to read sprite id u16")? as u32
            };
            sprite_ids.push(sid);
        }

        thing.frame_groups.push(LegacyFrameGroup {
            group_type,
            width,
            height,
            exact_size,
            layers,
            pattern_x,
            pattern_y,
            pattern_z,
            frames,
            is_animation,
            animation_mode,
            loop_count,
            start_frame,
            frame_durations,
            sprite_ids,
        });
    }

    Ok(())
}

