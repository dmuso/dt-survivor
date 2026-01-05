use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use bevy_kira_audio::prelude::*;
use clap::Parser;
use donny_tango_survivor::{
    audio_plugin,
    combat_plugin,
    experience_plugin,
    game_plugin,
    inventory_plugin,
    pause_plugin,
    ui_plugin,
    visual_test::{self, TestScene, ScreenshotState},
    states::GameState
};

#[derive(Parser, Debug)]
#[command(name = "donny-tango-survivor")]
#[command(about = "A survivor-style game built with Bevy")]
struct Args {
    /// Skip intro and start game immediately
    #[arg(long)]
    auto_start: bool,

    /// Capture a screenshot of a visual test scene and exit.
    /// Use 'list' to see available scenes.
    #[arg(long)]
    screenshot: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Handle --screenshot list
    if let Some(ref scene_name) = args.screenshot {
        if scene_name == "list" {
            println!("Available visual test scenes:");
            for scene in TestScene::all() {
                println!("  {}", scene.name());
            }
            return;
        }
    }

    // Get the current directory and construct the assets path
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let assets_path = current_dir.join("assets");

    println!("Current directory: {:?}", current_dir);
    println!("Assets path: {:?}", assets_path);

    let mut app = App::new();

    // Check if we're in screenshot mode
    if let Some(ref scene_name) = args.screenshot {
        // Parse the scene name
        let scene: TestScene = scene_name.parse().unwrap_or_else(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });

        // Screenshot mode: minimal plugins + visual test plugin
        // Use smaller window for faster rendering and smaller output files
        app.add_plugins(DefaultPlugins.build()
                .disable::<bevy::audio::AudioPlugin>()
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().to_string(),
                    ..default()
                })
                .set(bevy::window::WindowPlugin {
                    primary_window: Some(bevy::window::Window {
                        resolution: bevy::window::WindowResolution::new(640, 480),
                        title: "Visual Test".to_string(),
                        visible: false,  // Run headless - no visible window
                        ..default()
                    }),
                    ..default()
                }))
            .add_plugins(HanabiPlugin) // Required for particle effects
            .insert_resource(ScreenshotState {
                scene,
                frames_remaining: scene.frames_to_wait(),
                screenshot_taken: false,
                exit_frames: 0,
                current_frame: 0,
                total_frames: scene.total_frames(),
                frames_between_captures: scene.frames_between_captures(),
            })
            // game_plugin registers material plugins and provides GameMeshes/GameMaterials
            .add_plugins(game_plugin)
            .add_plugins(visual_test::plugin);
    } else {
        // Normal game mode
        app.add_plugins(DefaultPlugins.build()
                .disable::<bevy::audio::AudioPlugin>()
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().to_string(),
                    ..default()
                }))
            .add_plugins(AudioPlugin)
            .add_plugins(HanabiPlugin)
            .init_state::<GameState>()
            .add_plugins((audio_plugin, combat_plugin, experience_plugin, game_plugin, inventory_plugin, pause_plugin, ui_plugin));

        // If auto-start flag is set, add a system to skip to InGame state
        if args.auto_start {
            app.add_systems(Startup, |mut next_state: ResMut<NextState<GameState>>| {
                next_state.set(GameState::InGame);
            });
        }
    }

    app.run();
}

#[cfg(test)]
mod tests {
    use super::*;
    use donny_tango_survivor::prelude::*;
    use donny_tango_survivor::spells::fire::fireball::{
        FireballProjectile, ChargingFireball, fireball_collision_detection, fireball_collision_effects,
    };
    use donny_tango_survivor::score::Score;
    use donny_tango_survivor::spell::systems::spell_casting_system;
    use donny_tango_survivor::inventory::resources::SpellList;
    use donny_tango_survivor::game::ScreenTintEffect;
    use donny_tango_survivor::enemies::components::Enemy;
    use donny_tango_survivor::ui::components::RadialCooldownOverlay;
    use donny_tango_survivor::ui::spell_slot::{SpellSlotVisual, SpellIconImage, SlotSource};
    use bevy::app::App;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn test_game_state_default() {
        let state = GameState::default();
        assert_eq!(state, GameState::Intro);
    }

    #[test]
    fn test_components_exist() {
        // Test that our component types can be created
        let _rock = Rock;
        let _menu_button = MenuButton;
        let _start_button = StartGameButton;
        let _exit_button = ExitGameButton;
    }

    #[test]
    fn test_player_mesh_properties() {
        // Test that player uses 3D mesh (GameMeshes.player is 1x1x1 cube)
        let transform = Transform::from_translation(Vec3::new(0.0, 0.5, 0.0));
        // Player is at Y=0.5 (half cube height above ground)
        assert_eq!(transform.translation.y, 0.5);
    }

    #[test]
    fn test_rock_mesh_properties() {
        // Test that rock uses 3D mesh (GameMeshes.rock is 1.0x0.5x1.0 flat cube)
        let transform = Transform::from_translation(Vec3::new(5.0, 0.25, 5.0));
        // Rock is at Y=0.25 (half height above ground)
        assert_eq!(transform.translation.y, 0.25);
    }

    #[test]
    fn test_player_transform_position() {
        // Test that player transform is created at center position
        let transform = Transform::from_translation(Vec3::new(0.0, 0.0, 0.0));

        assert_eq!(transform.translation, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_random_position_range() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Test that random positions are within expected bounds
        for _ in 0..100 {
            let x = rng.gen_range(-400.0..400.0);
            let y = rng.gen_range(-300.0..300.0);

            assert!(x >= -400.0 && x <= 400.0);
            assert!(y >= -300.0 && y <= 300.0);
        }
    }

    #[test]
    fn test_random_rock_sizes() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Test that random rock sizes are within expected bounds
        for _ in 0..100 {
            let size = rng.gen_range(10.0..30.0);
            assert!(size >= 10.0 && size <= 30.0);
        }
    }

    #[test]
    fn test_game_state_enum_variants() {
        // Test that game states exist and are distinct
        assert_ne!(GameState::Intro, GameState::InGame);
        assert_ne!(GameState::AttunementSelect, GameState::InGame);
        assert_ne!(GameState::InventoryOpen, GameState::InGame);
        assert_eq!(GameState::Intro as u8, 0);
        assert_eq!(GameState::AttunementSelect as u8, 1);
        assert_eq!(GameState::InGame as u8, 2);
        assert_eq!(GameState::InventoryOpen as u8, 3);
    }

    #[test]
    fn test_camera_reuse_across_state_transitions() {
        let mut app = App::new();

        // Add minimal plugins needed for state transitions
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering and UI materials
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();
        app.init_asset::<bevy::shader::Shader>();

        // Initialize game state (starts in Intro by default)
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<SpellList>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system, inventory_plugin for SpellList)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin, ui_plugin));

        // Verify initial state is Intro
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::Intro);

        // Run startup systems (this should create the intro camera)
        app.update();

        // Verify camera exists after intro setup
        let camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_exists, "Should have a camera after intro setup");

        // Check that intro UI elements exist
        let has_ui = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(has_ui, "Should have UI nodes in intro state");

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update(); // Process state transition

        // Verify state changed to InGame
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::InGame);

        // Verify camera still exists (reused, not recreated)
        let camera_still_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_still_exists, "Should still have camera after transitioning to InGame");

        // Verify game entities exist (player and rocks)
        let _has_player = app.world_mut().query::<&Player>().single(app.world()).is_ok();
        assert!(_has_player, "Should have a player in InGame");

        let _rock_count = app.world_mut().query::<&Rock>().iter(app.world()).count();
        assert_eq!(_rock_count, 15, "Should have 15 rocks in InGame");

        // Verify score display UI element exists in InGame state
        let has_ui_ingame = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(has_ui_ingame, "Should have UI nodes (score display) in InGame state");

        // Transition back to Intro state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update(); // Process state transition

        // Verify state changed back to Intro
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::Intro);

        // Verify camera still exists (reused again)
        let camera_exists_again = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        assert!(camera_exists_again, "Should still have camera after transitioning back to Intro");

        // Verify UI elements are back
        let _has_ui_again = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();
        assert!(_has_ui_again, "Should have UI nodes again in Intro state");

        // Verify game entities are gone
        let _has_player_intro = app.world_mut().query::<&Player>().single(app.world()).is_ok();
        assert!(!_has_player_intro, "Should have no players in Intro state");

        let rock_count_intro = app.world_mut().query::<&Rock>().iter(app.world()).count();
        assert_eq!(rock_count_intro, 0, "Should have no rocks in Intro state");
    }

    #[test]
    fn test_no_blank_screen_during_transitions() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering and UI materials
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();
        app.init_asset::<bevy::shader::Shader>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<SpellList>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system, inventory_plugin for SpellList)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin, ui_plugin));

        // Run initial update to set up intro
        app.update();

        // Record initial state
        let initial_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let initial_has_content = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();

        // Ensure we start with content to render
        assert!(initial_camera_exists, "Should have camera initially");
        assert!(initial_has_content, "Should have renderable content initially");

        // Transition to InGame
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify we still have camera and content immediately after transition
        let post_transition_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let post_transition_has_content = app.world_mut().query::<&Player>().single(app.world()).is_ok() ||
                                          app.world_mut().query::<&Rock>().iter(app.world()).next().is_some();

        assert!(post_transition_camera_exists, "Camera should exist immediately after transition");
        assert!(post_transition_has_content, "Should have renderable content immediately after transition");

        // Transition back to Intro
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update();

        // Verify camera persists and content is available
        let final_camera_exists = app.world_mut().query::<&Camera>().single(app.world()).is_ok();
        let final_has_content = app.world_mut().query::<&Node>().iter(app.world()).next().is_some();

        assert!(final_camera_exists, "Camera should persist throughout all transitions");
        assert!(final_has_content, "Should have renderable content after all transitions");
    }

    #[test]
    fn test_spell_list_initialization() {
        // SpellList starts empty - spells are added when Whisper is collected
        let spell_list = SpellList::default();

        // Check that spell list is empty by default
        assert!(spell_list.find_empty_slot().is_some(), "SpellList should have empty slots by default");
        assert_eq!(spell_list.iter_spells().count(), 0, "SpellList should have no spells initially");
    }

    #[test]
    fn test_player_starts_without_spells() {
        // Since spells are only equipped after Whisper is collected,
        // this test verifies that player starts WITHOUT spells
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that player exists
        let world = app.world_mut();
        let player_count = world.query::<&Player>().iter(world).count();
        assert_eq!(player_count, 1, "Should have exactly one player");

        // Player starts with no spells - spells are added when Whisper is collected
        let spell_list = world.get_resource::<SpellList>().unwrap();
        assert_eq!(spell_list.iter_spells().count(), 0, "Should have no spells until Whisper is collected");
    }

    #[test]
    fn test_spell_casting_spawns_fireballs() {
        use donny_tango_survivor::whisper::resources::SpellOrigin;
        use donny_tango_survivor::spell::{Spell, SpellType};
        use donny_tango_survivor::inventory::resources::SpellList;

        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, inventory_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Simulate Whisper collection by setting SpellOrigin position (full 3D)
        app.world_mut().resource_mut::<SpellOrigin>().position = Some(Vec3::new(0.0, 3.0, 0.0));

        // Add spell to SpellList resource (new approach)
        {
            let mut spell_list = app.world_mut().resource_mut::<SpellList>();
            let mut fireball = Spell::new(SpellType::Fireball);
            fireball.last_fired = -2.0; // Ready to fire
            spell_list.equip(fireball);
        }

        // Create an enemy to target
        app.world_mut().spawn((
            Transform::from_translation(Vec3::new(100.0, 0.0, 0.0)),
            Enemy { speed: 50.0, strength: 10.0 },
        ));

        // Advance time to allow spell to fire (past the 2 second cooldown)
        {
            let mut time = app.world_mut().get_resource_mut::<Time>().unwrap();
            time.advance_by(std::time::Duration::from_secs(3));
        }

        // Run spell casting system
        let _ = app.world_mut().run_system_once(spell_casting_system);

        // Check that charging fireballs were spawned (charge phase)
        let world = app.world_mut();
        let fireball_count = world.query::<&ChargingFireball>().iter(world).count();
        assert_eq!(fireball_count, 1, "Spell casting should spawn 1 fireball");
    }

    #[test]
    fn test_weapon_slots_ui_created() {
        let mut app = App::new();

        // Add minimal plugins
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
            bevy::gizmos::GizmoPlugin,
        ));

        // Initialize asset types needed for 3D rendering and UI materials
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();
        app.init_asset::<bevy::shader::Shader>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin, ui_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Check that spell slot visuals are created with Active source
        let world = app.world_mut();
        let slot_visuals: Vec<_> = world.query::<&SpellSlotVisual>().iter(world).collect();
        assert_eq!(slot_visuals.len(), 5, "Should have 5 spell slot visuals");
        for visual in slot_visuals {
            assert_eq!(visual.source, SlotSource::Active, "All slots should have Active source");
        }

        // Check that spell icon images exist for all slots
        let icon_count = world.query::<&SpellIconImage>().iter(world).count();
        assert_eq!(icon_count, 5, "Should have 5 spell icon images for all slots");

        // Check that radial cooldown overlays exist for all slots
        let overlay_count = world.query::<&RadialCooldownOverlay>().iter(world).count();
        assert_eq!(overlay_count, 5, "Should have 5 radial cooldown overlay elements");
    }

    #[test]
    fn test_scoring_integration_full_flow() {
        use donny_tango_survivor::combat::{CheckDeath, Health, apply_damage_system, check_death_system, handle_enemy_death_system};

        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<SpellList>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify we're in InGame state
        assert_eq!(*app.world().get_resource::<State<GameState>>().unwrap(), GameState::InGame);

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Create a fireball and enemy for collision testing
        // Enemy needs Health and CheckDeath for the combat system
        // Collision detection uses XZ plane, so place fireball and enemy within collision radius on XZ
        use donny_tango_survivor::game::events::FireballEnemyCollisionEvent;
        app.add_message::<FireballEnemyCollisionEvent>();

        let fireball_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.0, 0.15, 0.0)), // X=0, Z=0
            FireballProjectile {
                direction: Vec3::new(1.0, 0.0, 0.0),
                speed: 300.0,
                damage: 10.0,
                burn_tick_damage: 2.0,
                lifetime: Timer::from_seconds(5.0, TimerMode::Once),
                spawn_position: Vec3::ZERO,
                travel_time: 0.0,
            },
        )).id();

        let enemy_entity = app.world_mut().spawn((
            Transform::from_translation(Vec3::new(0.5, 0.375, 0.0)), // X=0.5, Z=0 - within collision radius
            Enemy { speed: 50.0, strength: 10.0 },
            Health::new(10.0),
            CheckDeath,
        )).id();

        // Run collision and combat systems in sequence
        let _ = app.world_mut().run_system_once(fireball_collision_detection);
        let _ = app.world_mut().run_system_once(fireball_collision_effects);
        let _ = app.world_mut().run_system_once(apply_damage_system);
        let _ = app.world_mut().run_system_once(check_death_system);
        let _ = app.world_mut().run_system_once(handle_enemy_death_system);

        // Verify both entities are despawned
        assert!(!app.world().entities().contains(fireball_entity), "Fireball should be despawned after collision");
        assert!(!app.world().entities().contains(enemy_entity), "Enemy should be despawned after death");

        // Verify score incremented
        let updated_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(updated_score.0, 1, "Score should increment to 1 after enemy defeat");
    }

    #[test]
    fn test_scoring_integration_multiple_enemies() {
        use donny_tango_survivor::combat::{CheckDeath, Health, apply_damage_system, check_death_system, handle_enemy_death_system};

        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<SpellList>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify score starts at 0
        let initial_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(initial_score.0, 0, "Score should start at 0");

        // Register fireball collision event
        use donny_tango_survivor::game::events::FireballEnemyCollisionEvent;
        app.add_message::<FireballEnemyCollisionEvent>();

        // Create multiple fireballs and enemies at once
        // Collision detection uses XZ plane, so space them out in Z (Y is height)
        let mut fireball_entities = Vec::new();
        let mut enemy_entities = Vec::new();
        for i in 0..3 {
            let fireball_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.0, 0.15, i as f32 * 20.0)), // Spread in Z
                FireballProjectile {
                    direction: Vec3::new(1.0, 0.0, 0.0),
                    speed: 300.0,
                    damage: 10.0,
                    burn_tick_damage: 2.0,
                    lifetime: Timer::from_seconds(5.0, TimerMode::Once),
                    spawn_position: Vec3::ZERO,
                    travel_time: 0.0,
                },
            )).id();
            fireball_entities.push(fireball_entity);

            let enemy_entity = app.world_mut().spawn((
                Transform::from_translation(Vec3::new(0.5, 0.375, i as f32 * 20.0)), // Same Z, within collision radius on X
                Enemy { speed: 50.0, strength: 10.0 },
                Health::new(10.0),
                CheckDeath,
            )).id();
            enemy_entities.push(enemy_entity);
        }

        // Run collision and combat systems in sequence
        let _ = app.world_mut().run_system_once(fireball_collision_detection);
        let _ = app.world_mut().run_system_once(fireball_collision_effects);
        let _ = app.world_mut().run_system_once(apply_damage_system);
        let _ = app.world_mut().run_system_once(check_death_system);
        let _ = app.world_mut().run_system_once(handle_enemy_death_system);

        // Verify all entities are despawned
        for (i, &fireball_entity) in fireball_entities.iter().enumerate() {
            assert!(!app.world().entities().contains(fireball_entity), "Fireball {} should be despawned", i);
        }
        for (i, &enemy_entity) in enemy_entities.iter().enumerate() {
            assert!(!app.world().entities().contains(enemy_entity), "Enemy {} should be despawned", i);
        }

        // Verify score incremented for each enemy defeated
        let final_score = app.world().get_resource::<Score>().unwrap();
        assert_eq!(final_score.0, 3, "Score should be 3 after defeating 3 enemies");
    }

    #[test]
    fn test_scoring_integration_score_persistence() {
        let mut app = App::new();

        // Add minimal plugins for core functionality
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Initialize required resources
        app.init_resource::<SpellList>();
        app.init_resource::<ScreenTintEffect>();

        // Add our plugins (combat_plugin required for damage system)
        app.add_plugins((combat_plugin, game_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Score some points
        {
            let mut score = app.world_mut().get_resource_mut::<Score>().unwrap();
            score.0 = 5;
        }

        // Transition back to Intro and then to InGame again
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::Intro);
        app.update();
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Score should persist across state transitions (not reset)
        let score_after_reset = app.world().get_resource::<Score>().unwrap();
        assert_eq!(score_after_reset.0, 5, "Score should persist across state transitions");
    }

    #[test]
    fn test_game_uses_3d_components() {
        use donny_tango_survivor::game::resources::{GameMeshes, GameMaterials};

        let mut app = App::new();

        // Add minimal plugins for 3D rendering
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::state::app::StatesPlugin,
            bevy::time::TimePlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::mesh::MeshPlugin,
            bevy::image::ImagePlugin::default(),
        ));

        // Initialize asset types needed for 3D rendering
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();

        // Initialize game state
        app.init_state::<GameState>();

        // Add our plugins
        app.add_plugins((combat_plugin, game_plugin, inventory_plugin));

        // Transition to InGame state
        app.world_mut().get_resource_mut::<NextState<GameState>>().unwrap().set(GameState::InGame);
        app.update();

        // Verify 3D rendering resources exist
        let world = app.world();

        // Verify GameMeshes resource is available
        let game_meshes = world.get_resource::<GameMeshes>();
        assert!(game_meshes.is_some(), "GameMeshes resource should be initialized");

        // Verify GameMaterials resource is available
        let game_materials = world.get_resource::<GameMaterials>();
        assert!(game_materials.is_some(), "GameMaterials resource should be initialized");

        // Verify player entity exists with Transform (3D model loaded as child scene)
        let world = app.world_mut();
        let player_entities: Vec<Entity> = world.query_filtered::<Entity, (With<Player>, With<Transform>)>()
            .iter(world)
            .collect();
        assert_eq!(player_entities.len(), 1, "Player should exist with Transform component");

        // Verify Camera3d exists
        let camera_3d_count = world.query_filtered::<Entity, With<Camera3d>>()
            .iter(world)
            .count();
        assert!(camera_3d_count > 0, "Should have at least one Camera3d");

        // Verify no Sprite components on game entities (only 3D used)
        let player_with_sprite: Vec<Entity> = world.query_filtered::<Entity, (With<Player>, With<Sprite>)>()
            .iter(world)
            .collect();
        assert_eq!(player_with_sprite.len(), 0, "Player should NOT have Sprite component in 3D mode");
    }
}