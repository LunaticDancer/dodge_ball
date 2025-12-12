use bevy::math::FloatPow;
use bevy::{input::mouse::MouseMotion, prelude::*, window::WindowResized};
use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::{f32::consts::PI, time::Duration};

const MAIN_FONT_PATH: &str = "Doto_Rounded-Bold.ttf";
const PLAYER_MOVEMENT_SPEED_NORMALIZED: f32 = 0.5; // how much of the entire screen should the player travel per second
const BULLET_MOVEMENT_SPEED_NORMALIZED: f32 = 0.4;
const BULLET_COLOR_OSCILATION_SPEED: f32 = 108.;
const BULLET_PARTICLE_INTERVAL: f32 = 0.1;
const TRAIL_PARTICLE_LIFETIME: f32 = 0.7;
const COLLISION_PARTICLE_LIFETIME: f32 = 0.5;
const COLLISION_PARTICLE_COUNT: i32 = 32;
const COLLISION_PARTICLE_SPEED_NORMALIZED: f32 = 0.3;
const SCREENSHAKE_VELOCITY: f32 = 213.7;
const SCREENSHAKE_ON_SHOOT: f32 = 0.005;
const SCREENSHAKE_ON_BOUNCE: f32 = 0.003;
const SCREENSHAKE_ON_DEATH: f32 = 0.01;
const SCREENSHAKE_DAMPENING: f32 = 10.0;
const PLAYER_SIZE: f32 = 0.02;
const GAMEPAD_STICK_DEADZONE: f32 = 0.1;
const GAMEPAD_AIM_DEADZONE: f32 = 0.5;
const GAMEPAD_AIM_DISTANCE: f32 = 0.1;
const MOUSE_DEADZONE: f32 = 1.0;
const TEXT_COLOR: Color = Color::hsv(0.0, 0.0, 0.5);
const IDLE_BUTTON: Color = Color::hsv(0.0, 0.0, 1.0);
const HOVERED_BUTTON: Color = Color::hsv(0.0, 0.0, 0.2);
const PRESSED_BUTTON: Color = Color::hsv(0.0, 0.0, 0.6);

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Uninitialized,
    Menu,
    InGame,
    Paused,
    GameOver,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
enum ControlDevice {
    Keyboard,
    Gamepad,
    #[default]
    Mouse,
}

#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

#[derive(Resource)]
struct Score {
    value: f32,
}

#[derive(Resource)]
struct ScreenshakeIntensity {
    value: f32,
}

#[derive(Resource)]
struct BulletRenderComponents {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

#[derive(Resource)]
struct PrimaryControlDevice {
    value: ControlDevice,
}

#[derive(Resource)]
struct DisplayProperties {
    w: f32,
    h: f32,
    half_w: f32,
    half_h: f32,
    shorter_dimension: f32,
}

#[derive(Component)]
enum MenuButtonAction {
    Play,
    Quit,
    Resume,
    ToMenu,
}

#[derive(Component)]
struct SelectedOption;

#[derive(Component)]
struct Player {
    bullet_timer: f32,
}
#[derive(Component)]
struct TrailParticleSpawner {
    timer: Timer,
}

#[derive(Component)]
struct PlayerAim;

#[derive(Component)]
struct Bullet;

#[derive(Component)]
struct TrailParticle {
    lifetime: f32,
}

#[derive(Component)]
struct BounceParticle {
    lifetime: f32,
    velocity: Vec3,
}

#[derive(Component)]
struct ScreenEdgeBouncer {
    velocity: Vec3,
}

#[derive(Component)]
struct ButtonsHolder;

#[derive(Component)]
struct ScoreDisplay;

fn main() {
    let mut app = App::new();

    app.insert_resource(ClearColor(Color::linear_rgb(0.00, 0.00, 0.00)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "DODGE_BALL".into(),
                        resizable: false,
                        mode: bevy::window::WindowMode::BorderlessFullscreen(
                            MonitorSelection::Primary,
                        ),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        );
    app.insert_resource(DisplayProperties {
        w: 640.,
        h: 480.,
        half_w: 320.,
        half_h: 240.,
        shorter_dimension: 480.,
    });
    app.insert_resource(PrimaryControlDevice {
        value: ControlDevice::Keyboard,
    });
    app.insert_resource(Score { value: 0.0 });
    let seeded_rng = ChaCha8Rng::seed_from_u64(2137);
    app.insert_resource(RandomSource(seeded_rng));
    app.insert_resource(ScreenshakeIntensity { value: 0.0 });

    app.add_systems(Startup, init_bullet_data);
    app.add_systems(
        OnEnter(AppState::Menu),
        (
            main_menu_setup,
            despawn_player,
            despawn_player_aim,
            despawn_bullets,
            reset_score,
        ),
    );
    app.add_systems(OnEnter(AppState::GameOver), game_over_screen_setup);
    app.add_systems(OnEnter(AppState::Paused), pause_menu_setup);
    app.add_systems(
        OnExit(AppState::Menu),
        (
            spawn_player,
            spawn_player_aim,
            gameplay_ui_setup,
            init_bullet_data,
        ),
    );
    app.add_systems(OnEnter(AppState::InGame), make_mouse_invisible);
    app.add_systems(OnExit(AppState::InGame), make_mouse_visible);
    app.add_systems(PreUpdate, check_for_mouse_input);
    app.add_systems(
        Update,
        (
            (
                button_react_to_mouse_system,
                button_react_to_keyboard_or_gamepad_system,
                menu_action,
            )
                .run_if(in_state(AppState::Menu).or(in_state(AppState::Paused))),
            resize_screen_bounds,
            handle_game_pausing,
            spawn_bullet
                .after(init_bullet_data)
                .run_if(in_state(AppState::InGame)),
            handle_score.run_if(in_state(AppState::InGame)),
            oscilate_bullet_colors,
            handle_game_over_continue.run_if(in_state(AppState::GameOver)),
            spawn_bullet_trail,
            handle_trail_particles,
            handle_screenshake,
        ),
    );
    app.add_systems(
        PostUpdate,
        (app_init.run_if(run_once), button_handle_display),
    );
    app.add_systems(
        FixedUpdate,
        (
            move_player,
            clamp_player.after(move_player),
            move_player_aim,
            clamp_player_aim.after(move_player_aim),
            move_bouncers,
            handle_bullet_collision,
            handle_bounce_particles,
        ),
    );

    app.init_state::<AppState>();
    app.run();
}

fn app_init(mut commands: Commands, mut game_state: ResMut<NextState<AppState>>, mut window: Single<&mut Window>) {
    commands.spawn((Camera2d::default(), Msaa::Off));
    game_state.set(AppState::Menu);
    window.resolution.set_scale_factor_override(Some(1.0));
}

fn init_bullet_data(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    display_properties: Res<DisplayProperties>,
) {
    commands.insert_resource(BulletRenderComponents {
        mesh: meshes.add(Circle::new(
            display_properties.shorter_dimension * PLAYER_SIZE,
        )),
        material: materials.add(Color::hsv(1., 1., 1.)),
    });
}

fn resize_screen_bounds(
    mut resize_reader: MessageReader<WindowResized>,
    window: Single<&Window>,
    mut display_properties: ResMut<DisplayProperties>,
) {
    for _e in resize_reader.read() {
        let w = window.resolution.physical_width();
        let h = window.resolution.physical_height();

        display_properties.w = (w) as f32;
        display_properties.h = (h) as f32;
        display_properties.half_w = display_properties.w / 2.;
        display_properties.half_h = display_properties.h / 2.;
        display_properties.shorter_dimension = if display_properties.w < display_properties.h {
            display_properties.w
        } else {
            display_properties.h
        };
    }
}

fn handle_screenshake(
    mut screenshake: ResMut<ScreenshakeIntensity>,
    mut camera: Single<&mut Transform, With<Camera2d>>,
    time: Res<Time<Real>>,
    display_properties: Res<DisplayProperties>,
) {
    screenshake.value = screenshake
        .value
        .lerp(0.0, (time.delta_secs() * SCREENSHAKE_DAMPENING).min(1.0));
    let rotation = SCREENSHAKE_VELOCITY * time.elapsed_secs();
    let dir = Vec2::new(rotation.cos(), rotation.sin());
    camera.translation =
        Vec3::new(dir.x, dir.y, 0.0) * screenshake.value * display_properties.shorter_dimension;
}

fn reset_score(mut score: ResMut<Score>) {
    score.value = 0.;
}

fn handle_score(
    time: Res<Time<Virtual>>,
    mut score: ResMut<Score>,
    display: Query<&mut Text, With<ScoreDisplay>>,
) {
    score.value += time.delta_secs();
    let time_text: String = convert_time_to_text(score.value);

    for mut text in display.into_iter() {
        text.0 = time_text.clone();
    }
}

fn convert_time_to_text(time: f32) -> String {
    let mut time_text: String = "".to_string();

    let ms = (time * 100.) as u32;
    let s = (ms - (ms % 100)) / 100;
    let m = (s - (s % 60)) / 60;

    if m < 10 {
        time_text += "0";
    }
    time_text.push_str(&m.to_string());
    time_text += ":";
    if (s % 60) < 10 {
        time_text += "0";
    }
    time_text.push_str(&(s % 60).to_string());
    time_text += ":";
    if (ms % 100) < 10 {
        time_text += "0";
    }
    time_text.push_str(&(ms % 100).to_string());

    time_text
}

fn make_mouse_visible(mut cursor_options: Single<&mut bevy::window::CursorOptions>) {
    cursor_options.visible = true;
}
fn make_mouse_invisible(mut cursor_options: Single<&mut bevy::window::CursorOptions>) {
    cursor_options.visible = false;
}

fn handle_trail_particles(
    mut commands: Commands,
    particles: Query<(Entity, &mut Transform, &mut TrailParticle)>,
    time: Res<Time<Virtual>>,
) {
    for (entity, mut transform, mut particle) in particles {
        particle.lifetime -= time.delta_secs();
        if particle.lifetime < 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.scale = Vec3::ONE * 0.0.lerp(0.5, particle.lifetime / TRAIL_PARTICLE_LIFETIME);
    }
}

fn spawn_bullet_trail(
    mut commands: Commands,
    bullet_data: Res<BulletRenderComponents>,
    bullets: Query<(&Transform, &mut TrailParticleSpawner)>,
    time: Res<Time<Virtual>>,
) {
    for (transform, mut spawner) in bullets {
        spawner.timer.tick(time.delta());

        if !spawner.timer.just_finished() {
            continue;
        }

        let initial_position = transform.translation;

        commands.spawn((
            TrailParticle {
                lifetime: TRAIL_PARTICLE_LIFETIME,
            },
            Mesh2d(bullet_data.mesh.clone()),
            MeshMaterial2d(bullet_data.material.clone()),
            Transform::from_translation(initial_position),
        ));
    }
}

fn spawn_bullet(
    mut commands: Commands,
    bullet_data: Res<BulletRenderComponents>,
    mut timer: Single<&mut Player, With<Player>>,
    player: Single<&Transform, With<Player>>,
    aim: Single<&Transform, With<PlayerAim>>,
    time: Res<Time<Virtual>>,
    display_properties: Res<DisplayProperties>,
    mut screenshake: ResMut<ScreenshakeIntensity>,
    asset_server: Res<AssetServer>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut evw_rumble: MessageWriter<GamepadRumbleRequest>,
    score: Res<Score>,
) {
    timer.bullet_timer -= time.delta_secs();

    if timer.bullet_timer > 0.0 {
        return;
    }

    let initial_velocity = (aim.translation - player.translation).normalize();
    let initial_position = player.translation
        + (initial_velocity * PLAYER_SIZE * 3.0 * display_properties.shorter_dimension);

    commands.spawn((
        Bullet,
        TrailParticleSpawner {
            timer: Timer::new(
                Duration::from_secs_f32(BULLET_PARTICLE_INTERVAL),
                TimerMode::Repeating,
            ),
        },
        Mesh2d(bullet_data.mesh.clone()),
        MeshMaterial2d(bullet_data.material.clone()),
        Transform::from_translation(initial_position),
        ScreenEdgeBouncer {
            velocity: initial_velocity,
        },
    ));
    commands.spawn((
        AudioPlayer::new(asset_server.load("Boom29.wav")),
        PlaybackSettings::DESPAWN,
    ));
    screenshake.value += SCREENSHAKE_ON_SHOOT;

    for (entity, _gamepad) in &gamepads {
        evw_rumble.write(GamepadRumbleRequest::Add {
            gamepad: entity,
            duration: Duration::from_millis(100),
            intensity: GamepadRumbleIntensity {
                strong_motor: 0.1,
                weak_motor: 0.3,
            },
        });
    }

    timer.bullet_timer += 0.05.lerp(2.0, (score.value / 10.0).squared().min(1.0));
}

fn handle_bounce_particles(
    mut commands: Commands,
    particles: Query<(Entity, &mut Transform, &mut BounceParticle)>,
    time: Res<Time<Fixed>>,
    display_properties: Res<DisplayProperties>,
) {
    for (entity, mut transform, mut particle) in particles {
        particle.lifetime -= time.delta_secs();
        if particle.lifetime < 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.scale = Vec3::ONE
            * ((PI / 2.0).lerp(0.0, particle.lifetime / TRAIL_PARTICLE_LIFETIME)).cos()
            * 0.5;
        transform.translation += particle.velocity
            * ((PI / 2.0).lerp(0.0, particle.lifetime / TRAIL_PARTICLE_LIFETIME)).cos()
            * COLLISION_PARTICLE_SPEED_NORMALIZED
            * display_properties.shorter_dimension
            * time.delta_secs();
    }
}

fn handle_bullet_collision(
    mut commands: Commands,
    mut bullets: Query<(&Transform, &mut ScreenEdgeBouncer), With<Bullet>>,
    player: Single<&Transform, With<Player>>,
    mut game_state: ResMut<NextState<AppState>>,
    display_properties: Res<DisplayProperties>,
    mut time: ResMut<Time<Virtual>>,
    bullet_data: Res<BulletRenderComponents>,
    mut randomness: ResMut<RandomSource>,
    mut screenshake: ResMut<ScreenshakeIntensity>,
    asset_server: Res<AssetServer>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut evw_rumble: MessageWriter<GamepadRumbleRequest>,
) {
    let collision_distance = PLAYER_SIZE * 2.0 * display_properties.shorter_dimension;
    let circle = Circle::new(1.0);

    let mut iter = bullets.iter_combinations_mut();
    while let Some([(bullet, mut bouncer), (second, mut bouncerer)]) = iter.fetch_next() {
        if bullet.translation.distance(player.translation) < collision_distance {
            time.pause();
            game_state.set(AppState::GameOver);
            screenshake.value += SCREENSHAKE_ON_DEATH;
            commands.spawn((
                AudioPlayer::new(asset_server.load("Random32.wav")),
                PlaybackSettings::DESPAWN,
            ));
    
            for (entity, _gamepad) in &gamepads {
                evw_rumble.write(GamepadRumbleRequest::Add {
                    gamepad: entity,
                    duration: Duration::from_millis(200),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: 0.9,
                        weak_motor: 0.6,
                    },
                });
                evw_rumble.write(GamepadRumbleRequest::Add {
                    gamepad: entity,
                    duration: Duration::from_millis(400),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: 0.2,
                        weak_motor: 0.5,
                    },
                });
            }
        }
        if second.translation.distance(player.translation) < collision_distance {
            time.pause();
            game_state.set(AppState::GameOver);
            screenshake.value += SCREENSHAKE_ON_DEATH;
            commands.spawn((
                AudioPlayer::new(asset_server.load("Random32.wav")),
                PlaybackSettings::DESPAWN,
            ));
    
            for (entity, _gamepad) in &gamepads {
                evw_rumble.write(GamepadRumbleRequest::Add {
                    gamepad: entity,
                    duration: Duration::from_millis(200),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: 0.9,
                        weak_motor: 0.6,
                    },
                });
                evw_rumble.write(GamepadRumbleRequest::Add {
                    gamepad: entity,
                    duration: Duration::from_millis(400),
                    intensity: GamepadRumbleIntensity {
                        strong_motor: 0.2,
                        weak_motor: 0.5,
                    },
                });
            }
        }

        if bullet.translation.distance(second.translation) > collision_distance {
            continue;
        }
        if bullet.translation.distance(second.translation) < 1.0 {
            continue;
        }

        let average_position = (bullet.translation + second.translation) / 2.0;
        let dir = (bullet.translation - second.translation).normalize();
        bouncer.velocity = dir;
        bouncerer.velocity = -dir;

        screenshake.value += SCREENSHAKE_ON_BOUNCE;
        commands.spawn((
            AudioPlayer::new(asset_server.load("Ball_Flick.wav")),
            PlaybackSettings::DESPAWN,
        ));

        for _ in 0..COLLISION_PARTICLE_COUNT {
            let rng = &mut randomness.0;
            let vel = circle.sample_boundary(rng);
            commands.spawn((
                BounceParticle {
                    lifetime: COLLISION_PARTICLE_LIFETIME,
                    velocity: Vec3::new(vel.x, vel.y, 0.0),
                },
                Transform::from_translation(average_position),
                Mesh2d(bullet_data.mesh.clone()),
                MeshMaterial2d(bullet_data.material.clone()),
            ));
        }
    }
}

fn oscilate_bullet_colors(
    time: Res<Time<Real>>,
    bullet_data: Res<BulletRenderComponents>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mat: &mut ColorMaterial = materials.get_mut(bullet_data.material.id()).unwrap();
    mat.color = Color::hsv(
        time.elapsed_secs() * BULLET_COLOR_OSCILATION_SPEED,
        1.,
        0.75,
    );
}

fn move_bouncers(
    bullets: Query<(&mut Transform, &mut ScreenEdgeBouncer)>,
    fixed_time: Res<Time<Fixed>>,
    display_properties: Res<DisplayProperties>,
) {
    let w_margin = display_properties.half_w - PLAYER_SIZE * display_properties.shorter_dimension;
    let h_margin = display_properties.half_h - PLAYER_SIZE * display_properties.shorter_dimension;
    for (mut trans, mut bouncer) in bullets {
        trans.translation += bouncer.velocity
            * BULLET_MOVEMENT_SPEED_NORMALIZED
            * display_properties.shorter_dimension
            * fixed_time.delta_secs();

        if bouncer.velocity.x > 0.0 {
            if trans.translation.x > w_margin {
                bouncer.velocity.x = -bouncer.velocity.x;
            }
        } else {
            if trans.translation.x < -w_margin {
                bouncer.velocity.x = -bouncer.velocity.x;
            }
        }

        if bouncer.velocity.y > 0.0 {
            if trans.translation.y > h_margin {
                bouncer.velocity.y = -bouncer.velocity.y;
            }
        } else {
            if trans.translation.y < -h_margin {
                bouncer.velocity.y = -bouncer.velocity.y;
            }
        }
    }
}

fn despawn_bullets(mut commands: Commands, bullets: Query<(Entity, &Bullet)>) {
    for (entity_id, _) in bullets.iter() {
        commands.entity(entity_id).despawn();
    }
}

fn spawn_player_aim(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    display_properties: Res<DisplayProperties>,
) {
    let mesh = meshes.add(Circle::new(
        display_properties.shorter_dimension * PLAYER_SIZE * 0.5,
    ));
    let material = materials.add(Color::srgb(1., 1., 1.));
    commands.spawn((
        PlayerAim,
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(PLAYER_SIZE, PLAYER_SIZE, 1.)),
    ));
}

fn move_player_aim(
    mut motion: MessageReader<MouseMotion>,
    mut player_aim: Single<&mut Transform, With<PlayerAim>>,
    player: Single<&Transform, (With<Player>, Without<PlayerAim>)>,
    gamepads: Query<(Entity, &Gamepad)>,
    fixed_time: Res<Time<Fixed>>,
    display_properties: Res<DisplayProperties>,
) {
    let mut movement_vector = Vec2::ZERO;

    for mot in motion.read() {
        movement_vector += Vec2 {
            x: mot.delta.x,
            y: -mot.delta.y,
        };
    }

    player_aim.translation += vec3(movement_vector.x, movement_vector.y, 0.);

    for (_entity, gamepad) in &gamepads {
        movement_vector = Vec2 {
            x: gamepad.get(GamepadAxis::RightStickX).unwrap(),
            y: gamepad.get(GamepadAxis::RightStickY).unwrap(),
        };

        if movement_vector.length() < GAMEPAD_AIM_DEADZONE {
            continue;
        }

        let lerp_delta = 10.0 * fixed_time.delta_secs();
        player_aim.translation = player_aim.translation.lerp(
            player.translation
                + vec3(movement_vector.x, movement_vector.y, 0.)
                    * GAMEPAD_AIM_DISTANCE
                    * display_properties.shorter_dimension,
            if lerp_delta > 1.0 { 1.0 } else { lerp_delta },
        );
    }
}

fn clamp_player_aim(
    mut player: Single<&mut Transform, With<PlayerAim>>,
    display: Res<DisplayProperties>,
) {
    player.translation = Vec3 {
        x: player.translation.x.clamp(-display.half_w, display.half_w),
        y: player.translation.y.clamp(-display.half_h, display.half_h),
        z: 0.,
    }
}

fn despawn_player_aim(mut commands: Commands, players: Query<(Entity, &PlayerAim)>) {
    for (entity_id, _) in players.iter() {
        commands.entity(entity_id).despawn();
    }
}

fn despawn_player(mut commands: Commands, players: Query<(Entity, &Player)>) {
    for (entity_id, _) in players.iter() {
        commands.entity(entity_id).despawn();
    }
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    display_properties: Res<DisplayProperties>,
) {
    let mesh = meshes.add(Circle::new(
        display_properties.shorter_dimension * PLAYER_SIZE,
    ));

    let material = materials.add(Color::srgb(1., 1., 1.));
    commands.spawn((
        Player {
            bullet_timer: 2.0,
        },
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(0., 0., 0.)),
    ));
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player: Single<&mut Transform, With<Player>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut primary_device: ResMut<PrimaryControlDevice>,
    fixed_time: Res<Time<Fixed>>,
    display_properties: Res<DisplayProperties>,
) {
    let mut movement_vector = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW)
        || keyboard_input.pressed(KeyCode::ArrowUp)
        || keyboard_input.pressed(KeyCode::KeyZ)
    {
        movement_vector.y += 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }
    if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        movement_vector.y -= 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }
    if keyboard_input.pressed(KeyCode::KeyA)
        || keyboard_input.pressed(KeyCode::ArrowLeft)
        || keyboard_input.pressed(KeyCode::KeyQ)
    {
        movement_vector.x -= 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }
    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        movement_vector.x += 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }

    for (_entity, gamepad) in &gamepads {
        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > GAMEPAD_STICK_DEADZONE {
            movement_vector.x += left_stick_x;
            primary_device.value = ControlDevice::Gamepad;
        }
        let left_stick_y = gamepad.get(GamepadAxis::LeftStickY).unwrap();
        if left_stick_y.abs() > GAMEPAD_STICK_DEADZONE {
            movement_vector.y += left_stick_y;
            primary_device.value = ControlDevice::Gamepad;
        }
    }

    player.translation += vec3(movement_vector.x, movement_vector.y, 0.).clamp_length_max(1.0)
        * fixed_time.delta_secs()
        * PLAYER_MOVEMENT_SPEED_NORMALIZED
        * display_properties.shorter_dimension;
}

fn clamp_player(mut player: Single<&mut Transform, With<Player>>, display: Res<DisplayProperties>) {
    let ps = PLAYER_SIZE * display.shorter_dimension;
    player.translation = Vec3 {
        x: player.translation.x.clamp(-display.half_w + ps, display.half_w - ps),
        y: player.translation.y.clamp(-display.half_h + ps, display.half_h - ps),
        z: 0.,
    }
}

fn handle_game_pausing(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut primary_device: ResMut<PrimaryControlDevice>,
    mut time: ResMut<Time<Virtual>>,
    mut game_state: ResMut<NextState<AppState>>,
    state: Res<State<AppState>>,
) {
    let mut take_action: bool = false;
    if keyboard_input.just_pressed(KeyCode::Escape)
        || keyboard_input.just_pressed(KeyCode::Backspace)
    {
        take_action = true;
        primary_device.value = ControlDevice::Keyboard;
    }

    for (_entity, gamepad) in &gamepads {
        if take_action {
            break;
        }

        let just_pressed = gamepad.get_just_pressed().into_iter();
        for button in just_pressed {
            if *button == GamepadButton::Select || *button == GamepadButton::Start {
                take_action = true;
                primary_device.value = ControlDevice::Gamepad;
                break;
            }
        }
    }

    if take_action {
        if *state.get() == AppState::InGame {
            time.pause();
            game_state.set(AppState::Paused);
        } else if *state.get() == AppState::Paused {
            time.unpause();
            game_state.set(AppState::InGame);
        }
    }
}

fn handle_game_over_continue(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut primary_device: ResMut<PrimaryControlDevice>,
    mut game_state: ResMut<NextState<AppState>>,
    mut time: ResMut<Time<Virtual>>,
    mouse_press: Res<ButtonInput<MouseButton>>,
) {
    let mut take_action: bool = false;
    if keyboard_input.just_pressed(KeyCode::Escape)
        || keyboard_input.just_pressed(KeyCode::Backspace)
        || keyboard_input.just_pressed(KeyCode::Space)
        || keyboard_input.just_pressed(KeyCode::Enter)
    {
        take_action = true;
        primary_device.value = ControlDevice::Keyboard;
    }

    for (_entity, gamepad) in &gamepads {
        if take_action {
            break;
        }

        let just_pressed = gamepad.get_just_pressed().into_iter();
        for button in just_pressed {
            if *button == GamepadButton::Select
                || *button == GamepadButton::Start
                || *button == GamepadButton::South
                || *button == GamepadButton::East
            {
                take_action = true;
                primary_device.value = ControlDevice::Gamepad;
                break;
            }
        }
    }

    if mouse_press.just_pressed(MouseButton::Left) || mouse_press.just_pressed(MouseButton::Right) {
        take_action = true;
        primary_device.value = ControlDevice::Mouse;
    }

    if take_action {
        game_state.set(AppState::Menu);
        time.unpause();
    }
}

fn check_for_mouse_input(
    mut motion: MessageReader<MouseMotion>,
    mut primary_device: ResMut<PrimaryControlDevice>,
    time: Res<Time<Virtual>>,
) {
    for ev in motion.read() {
        if ev.delta.x + ev.delta.y > MOUSE_DEADZONE * time.delta_secs() {
            primary_device.value = ControlDevice::Mouse;
        }
    }
}

// This system handles changing all buttons color based on mouse interaction
fn button_react_to_mouse_system(
    mut commands: Commands,
    mut interaction_query: Query<(Entity, &Interaction, Option<&SelectedOption>), With<Button>>,
    selected_options: Query<Entity, With<SelectedOption>>,
    primary_device: Res<PrimaryControlDevice>,
) {
    if primary_device.value != ControlDevice::Mouse {
        return;
    }

    for (entity, interaction, selected) in &mut interaction_query {
        if *interaction == Interaction::None && selected.is_some() {
            if selected_options.count() > 1 {
                commands.entity(entity).remove::<SelectedOption>();
            }
        }
        if *interaction == Interaction::Hovered && selected.is_none() {
            commands.entity(entity).insert(SelectedOption);
        }
    }
}

fn button_react_to_keyboard_or_gamepad_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut commands: Commands,
    mut interaction_query: Query<(Entity, &Interaction, Option<&SelectedOption>), With<Button>>,
    button_holder_query: Query<(Entity, &Children), With<ButtonsHolder>>,
    mut primary_device: ResMut<PrimaryControlDevice>,
) {
    let mut movement_vector = Vec2::ZERO;
    let mut confirm_command: bool = false;

    if keyboard_input.just_pressed(KeyCode::KeyW)
        || keyboard_input.just_pressed(KeyCode::ArrowUp)
        || keyboard_input.just_pressed(KeyCode::KeyZ)
    {
        movement_vector.y += 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }
    if keyboard_input.just_pressed(KeyCode::KeyS) || keyboard_input.just_pressed(KeyCode::ArrowDown)
    {
        movement_vector.y -= 1.0;
        primary_device.value = ControlDevice::Keyboard;
    }

    if keyboard_input.just_pressed(KeyCode::Enter) || keyboard_input.just_pressed(KeyCode::Space) {
        confirm_command = true;
        primary_device.value = ControlDevice::Keyboard;
    }

    for (_entity, gamepad) in &gamepads {
        let just_pressed = gamepad.get_just_pressed().into_iter();
        for button in just_pressed {
            if *button == GamepadButton::South || *button == GamepadButton::East {
                confirm_command = true;
                primary_device.value = ControlDevice::Gamepad;
            }

            if *button == GamepadButton::DPadUp {
                movement_vector.y += 1.0;
                primary_device.value = ControlDevice::Gamepad;
            }
            if *button == GamepadButton::DPadDown {
                movement_vector.y -= 1.0;
                primary_device.value = ControlDevice::Gamepad;
            }
        }
    }

    for (_, children) in button_holder_query {
        let mut buttons: Vec<Entity> = Vec::new();
        let mut selected_index = 0;
        for child in children {
            for (entity, _, sel) in &mut interaction_query {
                if child != &entity {
                    continue;
                }
                if sel.is_some() {
                    selected_index = buttons.len(); // doing this before adding the element to avoid the subtract one necessity
                }

                buttons.push(entity);
            }
        }

        if buttons.len() == 0 {
            continue;
        }

        if movement_vector.y > GAMEPAD_STICK_DEADZONE {
            commands
                .entity(buttons[selected_index])
                .remove::<SelectedOption>();

            if selected_index == 0 {
                commands
                    .entity(*buttons.last().unwrap())
                    .insert(SelectedOption);
            } else {
                commands
                    .entity(buttons[selected_index - 1])
                    .insert(SelectedOption);
            }
        }

        if movement_vector.y < -GAMEPAD_STICK_DEADZONE {
            commands
                .entity(buttons[selected_index])
                .remove::<SelectedOption>();

            if selected_index == buttons.len() - 1 {
                commands.entity(buttons[0]).insert(SelectedOption);
            } else {
                commands
                    .entity(buttons[selected_index + 1])
                    .insert(SelectedOption);
            }
        }
    }

    if confirm_command {
        for (entity, _, selected) in &mut interaction_query {
            if selected.is_none() {
                continue;
            }

            commands.entity(entity).insert(Interaction::Pressed);
        }
    }
}

fn button_handle_display(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
        With<Button>,
    >,
) {
    for (interaction, mut background_color, selected) in &mut button_query {
        *background_color = match (*interaction, selected) {
            (Interaction::Pressed, Some(_)) => PRESSED_BUTTON.into(),
            (_, Some(_)) => HOVERED_BUTTON.into(),
            (_, _) => IDLE_BUTTON.into(),
        }
    }
}

fn menu_action(
    interaction_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut game_state: ResMut<NextState<AppState>>,
    mut time: ResMut<Time<Virtual>>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match menu_button_action {
                MenuButtonAction::Quit => {
                    app_exit_writer.write(AppExit::Success);
                }
                MenuButtonAction::Play => {
                    game_state.set(AppState::InGame);
                }
                MenuButtonAction::Resume => {
                    game_state.set(AppState::InGame);
                    time.unpause();
                }
                MenuButtonAction::ToMenu => {
                    game_state.set(AppState::Menu);
                    time.unpause();
                }
            }
        }
    }
}

fn main_menu_setup(
    mut commands: Commands,
    window: Single<&Window>,
    asset_server: Res<AssetServer>,
) {
    let w = window.resolution.physical_width();
    let h = window.resolution.physical_height();
    println!("{}x{}", w, h);

    let font: Handle<Font> = asset_server.load(MAIN_FONT_PATH);

    let button_node = Node {
        width: px(w / 4),
        height: px(h / 6),
        margin: UiRect::all(px(h / 32)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_text_font = TextFont {
        font: font.clone(),
        font_size: (h / 12) as f32,
        ..default()
    };

    commands.spawn((
        DespawnOnExit(AppState::Menu),
        Text::new("LunaticDancer, 2025"),
        TextFont {
            font: font.clone(),
            font_size: (h / 20) as f32,
            ..default()
        },
        TextColor(TEXT_COLOR),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(8),
            left: px(8),
            ..default()
        },
    ));

    commands.spawn((
        DespawnOnExit(AppState::Menu),
        Text::new("v: 1.0.1, made with Bevy"),
        TextFont {
            font: font.clone(),
            font_size: (h / 20) as f32,
            ..default()
        },
        TextColor(TEXT_COLOR),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(8),
            right: px(8),
            ..default()
        },
    ));

    commands.spawn((
        DespawnOnExit(AppState::Menu),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            // vertical layout box
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            ButtonsHolder,
            children![
                // game title
                (
                    Text::new("DODGE_BALL"),
                    TextFont {
                        font_size: (h / 4) as f32,
                        font: font.clone(),
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(px(h / 16)),
                        ..default()
                    },
                ),
                // play button
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Play,
                    SelectedOption,
                    children![(
                        Text::new("Play"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),]
                ),
                // exit button
                (
                    Button,
                    button_node,
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Quit,
                    children![(Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),]
                ),
            ]
        )],
    ));
}

fn pause_menu_setup(
    mut commands: Commands,
    window: Single<&Window>,
    asset_server: Res<AssetServer>,
) {
    let w = window.resolution.physical_width();
    let h = window.resolution.physical_height();

    let font: Handle<Font> = asset_server.load(MAIN_FONT_PATH);

    let button_node = Node {
        width: px(w / 4),
        height: px(h / 8),
        margin: UiRect::all(px(8)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_text_font = TextFont {
        font: font.clone(),
        font_size: (h / 14) as f32,
        ..default()
    };

    commands.spawn((
        DespawnOnExit(AppState::Paused),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            // vertical layout box
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            ButtonsHolder,
            children![
                // game title
                (
                    Text::new("PAUSED"),
                    TextFont {
                        font: font.clone(),
                        font_size: (h / 10) as f32,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(px(12)),
                        ..default()
                    },
                ),
                // resume button
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Resume,
                    SelectedOption,
                    children![(
                        Text::new("Resume"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),]
                ),
                // to menu button
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::ToMenu,
                    children![(
                        Text::new("To Menu"),
                        button_text_font.clone(),
                        TextColor(TEXT_COLOR),
                    ),]
                ),
                // exit button
                (
                    Button,
                    button_node,
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Quit,
                    children![(Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),]
                ),
            ]
        )],
    ));
}

fn game_over_screen_setup(
    mut commands: Commands,
    window: Single<&Window>,
    asset_server: Res<AssetServer>,
) {
    let h = window.resolution.physical_height();

    let font: Handle<Font> = asset_server.load(MAIN_FONT_PATH);

    commands.spawn((
        DespawnOnExit(AppState::GameOver),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new("GAME OVER"),
            TextFont {
                font: font.clone(),
                font_size: (h / 6) as f32,
                ..default()
            },
            TextColor(TEXT_COLOR),
            Node {
                margin: UiRect::all(px(12)),
                ..default()
            },
        ),],
    ));
}

fn gameplay_ui_setup(
    mut commands: Commands,
    window: Single<&Window>,
    asset_server: Res<AssetServer>,
) {
    let h = window.resolution.physical_height();

    let font: Handle<Font> = asset_server.load(MAIN_FONT_PATH);

    commands.spawn((
        DespawnOnEnter(AppState::Menu),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            // vertical layout box
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            children![
                // score display
                (
                    ScoreDisplay,
                    Node {
                        margin: UiRect::all(px(8)),
                        ..default()
                    },
                    Text::new("00:00:00"),
                    TextFont {
                        font: font.clone(),
                        font_size: (h / 8) as f32,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                ),
            ]
        )],
    ));
}
