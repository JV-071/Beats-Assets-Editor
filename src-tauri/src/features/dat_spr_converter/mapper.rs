use super::dat_reader::{LegacyCategory, LegacyFrameGroup, LegacyThingType};
use crate::core::protobuf::{
    Appearance, AppearanceFlagAutomap, AppearanceFlagBank, AppearanceFlagClothes, AppearanceFlagDefaultAction,
    AppearanceFlagHeight, AppearanceFlagHook, AppearanceFlagLenshelp, AppearanceFlagLight, AppearanceFlagMarket,
    AppearanceFlagShift, AppearanceFlagWrite, AppearanceFlagWriteOnce, AppearanceFlags, FrameGroup,
    SpriteAnimation, SpriteInfo, SpritePhase,
};

/// Maps a legacy ThingType to a modern Protocol Buffers Appearance
pub fn map_legacy_thing_to_appearance(thing: &LegacyThingType) -> Appearance {
    Appearance {
        id: Some(thing.id),
        name: if !thing.market.name.is_empty() {
            Some(thing.market.name.as_bytes().to_vec())
        } else {
            None
        },
        description: None,
        flags: Some(map_legacy_flags(thing)),
        frame_group: thing
            .frame_groups
            .iter()
            .enumerate()
            .map(|(idx, fg)| map_legacy_frame_group(fg, thing.category, idx))
            .collect(),
        sprite_data: Vec::new(),
    }
}

fn map_legacy_flags(thing: &LegacyThingType) -> AppearanceFlags {
    let mut flags = AppearanceFlags::default();

    if thing.is_ground {
        flags.bank = Some(AppearanceFlagBank {
            waypoints: Some(thing.ground_speed as u32),
        });
    }

    if thing.is_ground_border {
        flags.clip = Some(true);
    }
    if thing.is_on_bottom {
        flags.bottom = Some(true);
    }
    if thing.is_on_top {
        flags.top = Some(true);
    }
    if thing.is_container {
        flags.container = Some(true);
    }
    if thing.is_stackable {
        flags.cumulative = Some(true);
    }
    if thing.is_usable {
        flags.usable = Some(true);
    }
    if thing.is_force_use {
        flags.forceuse = Some(true);
    }
    if thing.is_multi_use {
        flags.multiuse = Some(true);
    }

    if thing.is_writable {
        flags.write = Some(AppearanceFlagWrite {
            max_text_length: Some(thing.max_read_write_chars as u32),
        });
    }

    if thing.is_writable_once {
        flags.write_once = Some(AppearanceFlagWriteOnce {
            max_text_length_once: Some(thing.max_read_chars as u32),
        });
    }

    if thing.is_fluid_container {
        flags.liquidcontainer = Some(true);
    }
    if thing.is_fluid {
        flags.liquidpool = Some(true);
    }
    if thing.is_unpassable {
        flags.unpass = Some(true);
    }
    if thing.is_unmoveable {
        flags.unmove = Some(true);
    }
    if thing.is_block_missile {
        flags.unsight = Some(true);
    }
    if thing.is_block_pathfind {
        flags.avoid = Some(true);
    }
    if thing.is_no_move_animation {
        flags.no_movement_animation = Some(true);
    }
    if thing.is_pickupable {
        flags.take = Some(true);
    }
    if thing.is_hangable {
        flags.hang = Some(true);
    }

    if thing.is_vertical || thing.is_horizontal {
        flags.hook = Some(AppearanceFlagHook {
            direction: Some(if thing.is_vertical { 1 } else { 2 }),
        });
    }

    if thing.is_rotatable {
        flags.rotate = Some(true);
    }

    if thing.has_light {
        flags.light = Some(AppearanceFlagLight {
            brightness: Some(thing.light.level as u32),
            color: Some(thing.light.color as u32),
        });
    }

    if thing.dont_hide {
        flags.dont_hide = Some(true);
    }
    if thing.is_translucent {
        flags.translucent = Some(true);
    }

    if thing.has_offset {
        flags.shift = Some(AppearanceFlagShift {
            x: Some(thing.offset.x as u32),
            y: Some(thing.offset.y as u32),
        });
    }

    if thing.has_elevation {
        flags.height = Some(AppearanceFlagHeight {
            elevation: Some(thing.elevation as u32),
        });
    }

    if thing.is_lying_object {
        flags.lying_object = Some(true);
    }
    if thing.is_animate_always {
        flags.animate_always = Some(true);
    }

    if thing.is_mini_map {
        flags.automap = Some(AppearanceFlagAutomap {
            color: Some(thing.mini_map_color as u32),
        });
    }

    if thing.is_lens_help {
        flags.lenshelp = Some(AppearanceFlagLenshelp {
            id: Some(thing.lens_help as u32),
        });
    }

    if thing.is_full_ground {
        flags.fullbank = Some(true);
    }
    if thing.is_ignore_look {
        flags.ignore_look = Some(true);
    }

    if thing.is_cloth {
        flags.clothes = Some(AppearanceFlagClothes {
            slot: Some(thing.cloth_slot as u32),
        });
    }

    if thing.is_market_item {
        flags.market = Some(AppearanceFlagMarket {
            category: Some(thing.market.category as i32),
            trade_as_object_id: Some(thing.market.trade_as as u32),
            show_as_object_id: Some(thing.market.show_as as u32),
        });
        if thing.market.restrict_profession > 0 {
            flags.restrict_to_vocation = vec![thing.market.restrict_profession as i32];
        }
        if thing.market.restrict_level > 0 {
            flags.minimum_level = Some(thing.market.restrict_level as u32);
        }
    }

    if thing.has_default_action {
        flags.default_action = Some(AppearanceFlagDefaultAction {
            action: Some(thing.default_action as i32),
        });
    }

    if thing.is_wrappable {
        flags.wrap = Some(true);
    }
    if thing.is_unwrappable {
        flags.unwrap = Some(true);
    }
    if thing.is_top_effect {
        flags.topeffect = Some(true);
    }

    flags
}

fn map_legacy_frame_group(fg: &LegacyFrameGroup, category: LegacyCategory, group_idx: usize) -> FrameGroup {
    let fixed_frame_group_id: i32 = match category {
        LegacyCategory::Outfit => {
            if fg.group_type == 0 || group_idx == 0 {
                0 // FIXED_FRAME_GROUP_OUTFIT_IDLE
            } else {
                1 // FIXED_FRAME_GROUP_OUTFIT_MOVING
            }
        }
        _ => 2, // FIXED_FRAME_GROUP_OBJECT_INITIAL
    };

    let animation = if fg.is_animation && !fg.frame_durations.is_empty() {
        Some(SpriteAnimation {
            synchronized: Some(fg.animation_mode == 1),
            loop_type: None,
            loop_count: if fg.loop_count > 0 {
                Some(fg.loop_count as u32)
            } else {
                None
            },
            sprite_phase: fg
                .frame_durations
                .iter()
                .map(|d| SpritePhase {
                    duration_min: Some(d.min),
                    duration_max: Some(d.max),
                })
                .collect(),
        })
    } else {
        None
    };

    let sprite_info = SpriteInfo {
        pattern_width: Some(fg.width as u32),
        pattern_height: Some(fg.height as u32),
        pattern_depth: Some(fg.pattern_z as u32),
        layers: Some(fg.layers as u32),
        sprite_id: fg.sprite_ids.clone(),
        bounding_square: if fg.exact_size > 0 {
            Some(fg.exact_size as u32)
        } else {
            None
        },
        animation,
        is_opaque: None,
        bounding_box_per_direction: Vec::new(),
    };

    FrameGroup {
        fixed_frame_group: Some(fixed_frame_group_id),
        id: Some(fg.group_type as u32),
        sprite_info: Some(sprite_info),
    }
}
