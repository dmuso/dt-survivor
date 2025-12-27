use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct PlayerPosition(pub Vec2);

#[derive(Resource)]
pub struct EnemySpawnState {
    pub time_since_last_spawn: f32,
    pub spawn_rate_per_second: f32,
    pub time_since_last_rate_increase: f32,
    pub rate_level: u32,
}

impl Default for EnemySpawnState {
    fn default() -> Self {
        Self {
            time_since_last_spawn: 0.0,
            spawn_rate_per_second: 1.25, // Start with 1.25 enemies per second
            time_since_last_rate_increase: 0.0,
            rate_level: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayerDamageTimer {
    pub time_since_last_damage: f32,
    pub has_taken_damage: bool,
}

#[derive(Resource, Default)]
pub struct ScreenTintEffect {
    pub remaining_duration: f32,
    pub color: Color,
}

/// Tracks how long the player has survived in the current game session
#[derive(Resource, Default)]
pub struct SurvivalTime(pub f32);

/// Shared mesh handles for all game entities to avoid recreating meshes
#[derive(Resource)]
pub struct GameMeshes {
    /// Player mesh (1.0 x 1.0 x 1.0 cube)
    pub player: Handle<Mesh>,
    /// Enemy mesh (0.75 x 0.75 x 0.75 cube)
    pub enemy: Handle<Mesh>,
    /// Bullet mesh (0.3 x 0.3 x 0.3 cube)
    pub bullet: Handle<Mesh>,
    /// Laser beam mesh (thin elongated cube: 0.1 x 0.1 x 1.0, scaled by length)
    pub laser: Handle<Mesh>,
    /// Rocket mesh (elongated cube: 0.15 x 0.15 x 0.4)
    pub rocket: Handle<Mesh>,
    /// Explosion mesh (sphere with radius 1.0, scaled by explosion radius)
    pub explosion: Handle<Mesh>,
    /// Target marker mesh (small flat cube: 0.3 x 0.05 x 0.3)
    pub target_marker: Handle<Mesh>,
    /// Small loot mesh for XP orbs (0.4 x 0.4 x 0.4 cube)
    pub loot_small: Handle<Mesh>,
    /// Medium loot mesh for health packs and powerups (0.5 x 0.5 x 0.5 cube)
    pub loot_medium: Handle<Mesh>,
    /// Large loot mesh for weapons (0.6 x 0.6 x 0.6 cube)
    pub loot_large: Handle<Mesh>,
    /// Rock mesh (1.0 x 0.5 x 1.0 flat cube)
    pub rock: Handle<Mesh>,
}

impl GameMeshes {
    pub fn new(meshes: &mut Assets<Mesh>) -> Self {
        Self {
            player: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            enemy: meshes.add(Cuboid::new(0.75, 0.75, 0.75)),
            bullet: meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
            laser: meshes.add(Cuboid::new(0.1, 0.1, 1.0)),
            rocket: meshes.add(Cuboid::new(0.15, 0.15, 0.4)),
            explosion: meshes.add(Sphere::new(1.0)),
            target_marker: meshes.add(Cuboid::new(0.3, 0.05, 0.3)),
            loot_small: meshes.add(Cuboid::new(0.4, 0.4, 0.4)),
            loot_medium: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
            loot_large: meshes.add(Cuboid::new(0.6, 0.6, 0.6)),
            rock: meshes.add(Cuboid::new(1.0, 0.5, 1.0)),
        }
    }
}

/// Shared material handles for all game entities
#[derive(Resource)]
pub struct GameMaterials {
    /// Player material (green with slight emissive)
    pub player: Handle<StandardMaterial>,
    /// Enemy material (red)
    pub enemy: Handle<StandardMaterial>,
    /// Bullet material (yellow with emissive)
    pub bullet: Handle<StandardMaterial>,
    /// Laser beam material (cyan with strong emissive glow)
    pub laser: Handle<StandardMaterial>,
    /// Rocket pausing material (grey)
    pub rocket_pausing: Handle<StandardMaterial>,
    /// Rocket targeting material (yellow with emissive)
    pub rocket_targeting: Handle<StandardMaterial>,
    /// Rocket homing material (orange with emissive)
    pub rocket_homing: Handle<StandardMaterial>,
    /// Rocket exploding material (red with strong emissive)
    pub rocket_exploding: Handle<StandardMaterial>,
    /// Explosion material (red/orange with transparency and emissive)
    pub explosion: Handle<StandardMaterial>,
    /// Target marker material (red)
    pub target_marker: Handle<StandardMaterial>,
    /// XP orb material (light grey)
    pub xp_orb: Handle<StandardMaterial>,
    /// Health pack material (green)
    pub health_pack: Handle<StandardMaterial>,
    /// Pistol weapon loot material (yellow)
    pub weapon_pistol: Handle<StandardMaterial>,
    /// Laser weapon loot material (blue)
    pub weapon_laser: Handle<StandardMaterial>,
    /// Rocket launcher weapon loot material (orange)
    pub weapon_rocket: Handle<StandardMaterial>,
    /// Powerup material (magenta)
    pub powerup: Handle<StandardMaterial>,
    /// Rock obstacle material (grey)
    pub rock: Handle<StandardMaterial>,
}

impl GameMaterials {
    pub fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            player: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.0, 0.2, 0.0),
                ..default()
            }),
            enemy: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                ..default()
            }),
            bullet: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.5, 0.5, 0.0),
                ..default()
            }),
            laser: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 1.0),
                emissive: bevy::color::LinearRgba::rgb(0.0, 2.0, 2.0),
                unlit: true,
                ..default()
            }),
            rocket_pausing: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            }),
            rocket_targeting: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.3, 0.3, 0.0),
                ..default()
            }),
            rocket_homing: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.3, 0.15, 0.0),
                ..default()
            }),
            rocket_exploding: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(1.0, 0.0, 0.0),
                ..default()
            }),
            explosion: materials.add(StandardMaterial {
                base_color: Color::srgba(1.0, 0.3, 0.0, 0.8),
                emissive: bevy::color::LinearRgba::rgb(1.0, 0.2, 0.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
            target_marker: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: bevy::color::LinearRgba::rgb(0.5, 0.0, 0.0),
                ..default()
            }),
            xp_orb: materials.add(StandardMaterial {
                base_color: Color::srgb(0.75, 0.75, 0.75),
                ..default()
            }),
            health_pack: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                ..default()
            }),
            weapon_pistol: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 1.0, 0.0),
                ..default()
            }),
            weapon_laser: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 0.0, 1.0),
                ..default()
            }),
            weapon_rocket: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                ..default()
            }),
            powerup: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 1.0),
                ..default()
            }),
            rock: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.5),
                ..default()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_survival_time_default() {
        let time = SurvivalTime::default();
        assert_eq!(time.0, 0.0);
    }

    #[test]
    fn test_survival_time_increment() {
        let mut time = SurvivalTime::default();
        time.0 += 1.5;
        assert_eq!(time.0, 1.5);
    }

    mod game_meshes_tests {
        use super::*;
        use bevy::asset::Assets;
        use bevy::pbr::StandardMaterial;

        fn setup_test_app() -> App {
            let mut app = App::new();
            app.add_plugins(bevy::asset::AssetPlugin::default());
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app
        }

        #[test]
        fn test_game_meshes_has_all_required_handles() {
            let mut app = setup_test_app();
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();

            let game_meshes = GameMeshes::new(&mut meshes);

            // Verify all handles can retrieve their assets (are strong handles)
            assert!(meshes.get(&game_meshes.player).is_some());
            assert!(meshes.get(&game_meshes.enemy).is_some());
            assert!(meshes.get(&game_meshes.bullet).is_some());
            assert!(meshes.get(&game_meshes.laser).is_some());
            assert!(meshes.get(&game_meshes.rocket).is_some());
            assert!(meshes.get(&game_meshes.explosion).is_some());
            assert!(meshes.get(&game_meshes.target_marker).is_some());
            assert!(meshes.get(&game_meshes.loot_small).is_some());
            assert!(meshes.get(&game_meshes.loot_medium).is_some());
            assert!(meshes.get(&game_meshes.loot_large).is_some());
            assert!(meshes.get(&game_meshes.rock).is_some());
        }

        #[test]
        fn test_game_materials_has_all_required_handles() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let game_materials = GameMaterials::new(&mut materials);

            // Verify all handles can retrieve their assets (are strong handles)
            assert!(materials.get(&game_materials.player).is_some());
            assert!(materials.get(&game_materials.enemy).is_some());
            assert!(materials.get(&game_materials.bullet).is_some());
            assert!(materials.get(&game_materials.laser).is_some());
            assert!(materials.get(&game_materials.rocket_pausing).is_some());
            assert!(materials.get(&game_materials.rocket_targeting).is_some());
            assert!(materials.get(&game_materials.rocket_homing).is_some());
            assert!(materials.get(&game_materials.rocket_exploding).is_some());
            assert!(materials.get(&game_materials.explosion).is_some());
            assert!(materials.get(&game_materials.target_marker).is_some());
            assert!(materials.get(&game_materials.xp_orb).is_some());
            assert!(materials.get(&game_materials.health_pack).is_some());
            assert!(materials.get(&game_materials.weapon_pistol).is_some());
            assert!(materials.get(&game_materials.weapon_laser).is_some());
            assert!(materials.get(&game_materials.weapon_rocket).is_some());
            assert!(materials.get(&game_materials.powerup).is_some());
            assert!(materials.get(&game_materials.rock).is_some());
        }

        #[test]
        fn test_game_materials_colors_match_expected_values() {
            let mut app = setup_test_app();
            let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();

            let game_materials = GameMaterials::new(&mut materials);

            // Verify player is green
            let player_mat = materials.get(&game_materials.player).unwrap();
            assert_eq!(player_mat.base_color, Color::srgb(0.0, 1.0, 0.0));

            // Verify enemy is red
            let enemy_mat = materials.get(&game_materials.enemy).unwrap();
            assert_eq!(enemy_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify bullet is yellow with emissive
            let bullet_mat = materials.get(&game_materials.bullet).unwrap();
            assert_eq!(bullet_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify laser is cyan with emissive
            let laser_mat = materials.get(&game_materials.laser).unwrap();
            assert_eq!(laser_mat.base_color, Color::srgb(0.0, 1.0, 1.0));
            assert!(laser_mat.unlit);

            // Verify rocket_pausing is grey
            let rocket_pausing_mat = materials.get(&game_materials.rocket_pausing).unwrap();
            assert_eq!(rocket_pausing_mat.base_color, Color::srgb(0.5, 0.5, 0.5));

            // Verify rocket_targeting is yellow
            let rocket_targeting_mat = materials.get(&game_materials.rocket_targeting).unwrap();
            assert_eq!(rocket_targeting_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify rocket_homing is orange
            let rocket_homing_mat = materials.get(&game_materials.rocket_homing).unwrap();
            assert_eq!(rocket_homing_mat.base_color, Color::srgb(1.0, 0.5, 0.0));

            // Verify rocket_exploding is red
            let rocket_exploding_mat = materials.get(&game_materials.rocket_exploding).unwrap();
            assert_eq!(rocket_exploding_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify explosion is orange with transparency
            let explosion_mat = materials.get(&game_materials.explosion).unwrap();
            assert_eq!(explosion_mat.base_color, Color::srgba(1.0, 0.3, 0.0, 0.8));
            assert_eq!(explosion_mat.alpha_mode, AlphaMode::Blend);

            // Verify target_marker is red
            let target_marker_mat = materials.get(&game_materials.target_marker).unwrap();
            assert_eq!(target_marker_mat.base_color, Color::srgb(1.0, 0.0, 0.0));

            // Verify xp_orb is light grey
            let xp_mat = materials.get(&game_materials.xp_orb).unwrap();
            assert_eq!(xp_mat.base_color, Color::srgb(0.75, 0.75, 0.75));

            // Verify health_pack is green
            let health_mat = materials.get(&game_materials.health_pack).unwrap();
            assert_eq!(health_mat.base_color, Color::srgb(0.0, 1.0, 0.0));

            // Verify weapon_pistol is yellow
            let pistol_mat = materials.get(&game_materials.weapon_pistol).unwrap();
            assert_eq!(pistol_mat.base_color, Color::srgb(1.0, 1.0, 0.0));

            // Verify weapon_laser is blue
            let laser_mat = materials.get(&game_materials.weapon_laser).unwrap();
            assert_eq!(laser_mat.base_color, Color::srgb(0.0, 0.0, 1.0));

            // Verify weapon_rocket is orange
            let rocket_mat = materials.get(&game_materials.weapon_rocket).unwrap();
            assert_eq!(rocket_mat.base_color, Color::srgb(1.0, 0.5, 0.0));

            // Verify powerup is magenta
            let powerup_mat = materials.get(&game_materials.powerup).unwrap();
            assert_eq!(powerup_mat.base_color, Color::srgb(1.0, 0.0, 1.0));

            // Verify rock is grey
            let rock_mat = materials.get(&game_materials.rock).unwrap();
            assert_eq!(rock_mat.base_color, Color::srgb(0.5, 0.5, 0.5));
        }

        #[test]
        fn test_setup_game_assets_inserts_resources() {
            use crate::states::GameState;
            use crate::game::systems::setup_game_assets;

            let mut app = App::new();
            app.add_plugins((
                bevy::asset::AssetPlugin::default(),
                bevy::state::app::StatesPlugin,
            ));
            app.init_asset::<Mesh>();
            app.init_asset::<StandardMaterial>();
            app.init_state::<GameState>();
            app.add_systems(OnEnter(GameState::InGame), setup_game_assets);

            // Transition to InGame
            app.world_mut()
                .get_resource_mut::<bevy::state::state::NextState<GameState>>()
                .unwrap()
                .set(GameState::InGame);
            app.update();
            app.update();

            // Verify resources were inserted
            assert!(app.world().get_resource::<GameMeshes>().is_some());
            assert!(app.world().get_resource::<GameMaterials>().is_some());
        }
    }
}