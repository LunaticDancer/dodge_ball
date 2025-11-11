use bevy::{prelude::*, window::WindowResized};

const MAIN_FONT_PATH: &str = "Doto_Rounded-Bold.ttf";
const PLAYER_MOVEMENT_SPEED_NORMALIZED: f32 = 0.5; // how much of the entire screen should the player travel per second
const PLAYER_SIZE: f32 = 0.02;
const GAMEPAD_STICK_DEADZONE: f32 = 0.1;
const TEXT_COLOR: Color = Color::hsv(0.0, 0.0, 0.5);
const IDLE_BUTTON: Color = Color::hsv(0.0, 0.0, 1.0);
const HOVERED_BUTTON: Color = Color::hsv(0.0, 0.0, 0.8);
const PRESSED_BUTTON: Color = Color::hsv(0.0, 0.0, 0.6);

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame,
    Paused,
}

enum ControlDevice {
    Keyboard,
    Gamepad,
}

#[derive(Resource)]
struct Score {
    value: f32,
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
struct Player;

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

    app.add_systems(Startup, app_init);
    app.add_systems(OnEnter(AppState::Menu), (main_menu_setup, despawn_player));
    app.add_systems(OnEnter(AppState::Paused), pause_menu_setup);
    app.add_systems(OnExit(AppState::Menu), (spawn_player, gameplay_ui_setup));
    app.add_systems(
        Update,
        (
            (button_system, menu_action)
                .run_if(in_state(AppState::Menu).or(in_state(AppState::Paused))),
            resize_screen_bounds,
            handle_game_pausing,
            handle_score.run_if(in_state(AppState::InGame)),
        ),
    );
    app.add_systems(FixedUpdate, (move_player, clamp_player.after(move_player)));

    app.init_state::<AppState>();
    app.run();
}

fn app_init(mut commands: Commands) {
    commands.spawn((Camera2d::default(), Msaa::Off));
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

fn handle_score(time: Res<Time<Virtual>>, mut score: ResMut<Score>, mut display: Query<&mut Text, With<ScoreDisplay>>) {
    score.value += time.delta_secs();

    let ms = (score.value * 100.) as u32;
    let s = (ms - (ms % 100))/100;
    let m = (s - (s % 60))/60;

    let mut time_text: String = "".to_string();
    if m<10
    {
        time_text += "0";
    }
    time_text.push_str(&m.to_string());
    time_text += ":";
    if (s%60)<10
    {
        time_text += "0";
    }
    time_text.push_str(&(s%60).to_string());
    time_text += ":";
    if (ms%100)<10
    {
        time_text += "0";
    }
    time_text.push_str(&(ms%100).to_string());

    for mut text in display.into_iter()
    {
        text.0 = time_text.clone();
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
        Player,
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
    player.translation = Vec3 {
        x: player.translation.x.clamp(-display.half_w, display.half_w),
        y: player.translation.y.clamp(-display.half_h, display.half_h),
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

// This system handles changing all buttons color based on mouse interaction
fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&SelectedOption>),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut background_color, selected) in &mut interaction_query {
        *background_color = match (*interaction, selected) {
            (Interaction::Pressed, _) | (Interaction::None, Some(_)) => PRESSED_BUTTON.into(),
            (Interaction::Hovered, Some(_)) => PRESSED_BUTTON.into(),
            (Interaction::Hovered, None) => HOVERED_BUTTON.into(),
            (Interaction::None, None) => IDLE_BUTTON.into(),
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
                    children![(
                        Text::new("Quit"),
                        button_text_font,
                        TextColor(TEXT_COLOR),
                    ),]
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
                    children![(
                        Text::new("Quit"),
                        button_text_font,
                        TextColor(TEXT_COLOR),
                    ),]
                ),
            ]
        )],
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
