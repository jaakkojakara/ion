use std::sync::OnceLock;

use ion_common::Map;
use ion_engine::{
    core::coordinates::Location,
    gfx::{GfxBundle, GfxLight, Sprite, SpriteTypeId, textures::texture_assets::TextureAssets},
};

static GFX_REFS: OnceLock<Map<String, u32>> = OnceLock::new();

pub fn gfx_ref(ref_name: &str) -> u32 {
    *GFX_REFS.get().unwrap().get(ref_name).unwrap()
}

#[repr(u8)]
pub enum SpriteLayer {
    GroundBase = 1,
    GroundFade = 2,
    GroundDecal = 3,
    Entity = 4,
    Light = 10,
}

pub fn texture_assets() -> TextureAssets {
    let mut assets = TextureAssets::new();
    let mut gfx_refs = Map::default();

    let mut add_sprite_bundle = |name: &str, sprite_bundle: GfxBundle| {
        let id = assets.include_gfx_bundle(sprite_bundle);
        gfx_refs.insert(name.to_string(), id);
    };

    add_sprite_bundle(
        "test_tile_dark",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "test_tile_dark".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundBase as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "test_tile",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "test_tile".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundBase as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "warn_tile",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "warn_tile".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundBase as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "blue_tile",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "blue_tile".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundBase as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "grass",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "grass".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundBase as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "lamp",
        GfxBundle::new_with_lights(
            vec![
                Sprite::new(
                    SpriteTypeId::Normal,
                    "lamp".to_string(),
                    None,
                    Location { x: 0.1, y: 0.1 },
                    0.0,
                    1.0,
                    SpriteLayer::Entity as u8,
                    None,
                    false,
                ),
                Sprite::new(
                    SpriteTypeId::Light,
                    "lamp_light".to_string(),
                    None,
                    Location { x: 0.1, y: 0.1 },
                    0.0,
                    1.0,
                    SpriteLayer::Entity as u8,
                    None,
                    false,
                ),
            ],
            vec![GfxLight::new(
                Location::new(0.3, 0.3),
                5.0,
                Sprite::new(
                    SpriteTypeId::Light,
                    "light".to_string(),
                    None,
                    Location::new(-7.0, -7.0),
                    0.0,
                    2.0,
                    SpriteLayer::Light as u8,
                    None,
                    false,
                ),
            )],
        ),
    );

    add_sprite_bundle(
        "torus",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "torus".to_string(),
            None,
            Location { x: 0.5, y: 0.5 },
            0.0,
            2.0,
            SpriteLayer::Entity as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "stump",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "stump".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundDecal as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "block",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "block".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundDecal as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "bollard",
        GfxBundle::new_with_shadows(
            vec![Sprite::new(
                SpriteTypeId::Normal,
                "bollard".to_string(),
                None,
                Location { x: 0.0, y: 0.0 },
                0.0,
                1.0,
                SpriteLayer::GroundDecal as u8,
                None,
                false,
            )],
            vec![Sprite::new(
                SpriteTypeId::Shadow,
                "bollard_shadow".to_string(),
                None,
                Location { x: 0.07, y: -0.07 },
                0.0,
                1.0,
                SpriteLayer::GroundDecal as u8,
                None,
                false,
            )],
        ),
    );

    add_sprite_bundle(
        "shrub_1",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "shrub_1".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundDecal as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "shrub_2",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "shrub_2".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundDecal as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "shrub_3",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "shrub_3".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.0,
            SpriteLayer::GroundDecal as u8,
            None,
            false,
        )]),
    );

    add_sprite_bundle(
        "tree_big",
        GfxBundle::new(vec![Sprite::new(
            SpriteTypeId::Normal,
            "tree_big".to_string(),
            None,
            Location { x: 0.0, y: 0.0 },
            0.0,
            1.5,
            SpriteLayer::Entity as u8,
            None,
            false,
        )]),
    );

    for i in 0..8 {
        add_sprite_bundle(
            &format!("mc_idle_{}", i),
            GfxBundle::new_with_shadows(
                vec![Sprite::new(
                    SpriteTypeId::Normal,
                    format!("mc_idle_{}", i).to_string(),
                    None,
                    Location { x: -0.3, y: -0.3 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
                vec![Sprite::new(
                    SpriteTypeId::Shadow,
                    format!("mc_idle_shadow_{}", i).to_string(),
                    None,
                    Location { x: -0.2, y: -0.475 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
            ),
        );
    }

    for i in 0..8 {
        add_sprite_bundle(
            &format!("mc_run_{}", i),
            GfxBundle::new_with_shadows(
                vec![Sprite::new(
                    SpriteTypeId::Normal,
                    format!("mc_run_{}", i).to_string(),
                    None,
                    Location { x: -0.5, y: -0.5 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
                vec![Sprite::new(
                    SpriteTypeId::Shadow,
                    format!("mc_run_shadow_{}", i).to_string(),
                    None,
                    Location { x: -0.55, y: -0.64 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
            ),
        );
    }

    for i in 0..8 {
        add_sprite_bundle(
            &format!("z_idle_{}", i),
            GfxBundle::new_with_shadows(
                vec![Sprite::new(
                    SpriteTypeId::Normal,
                    format!("z_idle_{}", i).to_string(),
                    None,
                    Location { x: -0.3, y: -0.3 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
                vec![Sprite::new(
                    SpriteTypeId::Shadow,
                    format!("z_idle_shadow_{}", i).to_string(),
                    None,
                    Location { x: -0.2, y: -0.475 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
            ),
        );
    }

    for i in 0..8 {
        add_sprite_bundle(
            &format!("z_run_{}", i),
            GfxBundle::new_with_shadows(
                vec![Sprite::new(
                    SpriteTypeId::Normal,
                    format!("z_run_{}", i).to_string(),
                    None,
                    Location { x: -0.5, y: -0.5 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
                vec![Sprite::new(
                    SpriteTypeId::Shadow,
                    format!("z_run_shadow_{}", i).to_string(),
                    None,
                    Location { x: -0.55, y: -0.64 },
                    0.0,
                    1.0,
                    3,
                    Some(2),
                    true,
                )],
            ),
        );
    }

    for i in 0..256 {
        add_sprite_bundle(
            &format!("noise_{}", i),
            GfxBundle::new(vec![Sprite::new(
                SpriteTypeId::Normal,
                format!("noise_{}", i).to_string(),
                None,
                Location { x: 0.0, y: 0.0 },
                0.0,
                8.0,
                SpriteLayer::GroundBase as u8,
                None,
                false,
            )]),
        );
    }

    GFX_REFS.set(gfx_refs).unwrap();

    assets
}
