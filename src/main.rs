use bevy::prelude::*;

const PLAYER_MOVEMENT_SPEED:f32 = 160.;
const GAMEPAD_STICK_DEADZONE:f32 = 0.1;
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

#[derive(Component)]
pub struct Player;

pub enum ControlDevice
{
    Keyboard,
    Gamepad,
}

#[derive(Resource)]
pub struct PrimaryControlDevice
{
    value: ControlDevice,
}

#[derive(Component)]
enum MenuButtonAction {
    Play,
    Quit,
}

#[derive(Component)]
struct SelectedOption;

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
                .set(ImagePlugin::default_nearest())
        );
    
    app.add_systems(Startup, app_init);
    app.add_systems(OnEnter(AppState::Menu), main_menu_setup);
    app.add_systems(
        Update,
        (button_system, menu_action).run_if(in_state(AppState::Menu)),
    );

    app.init_state::<AppState>();
    app.run();
}

fn app_init(mut commands: Commands) {

    commands.spawn((
        Camera2d::default(),
        Msaa::Off,
));
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
            }
        }
    }
}

fn main_menu_setup(mut commands: Commands, window: Single<&Window>,) {
    let w = window.resolution.physical_width();
    let h = window.resolution.physical_height();

    let button_node = Node {
        width: px(w/4),
        height: px(h/6),
        margin: UiRect::all(px(h/32)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let button_text_font = TextFont {
        font_size: (h/12) as f32,
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
                        font_size: (h/4) as f32,
                        ..default()
                    },
                    TextColor(TEXT_COLOR),
                    Node {
                        margin: UiRect::all(px(h/16)),
                        ..default()
                    },
                ),
                // play button
                (
                    Button,
                    button_node.clone(),
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Play,
                    children![
                        (
                            Text::new("Play"),
                            button_text_font.clone(),
                            TextColor(TEXT_COLOR),
                        ),
                    ]
                ),
                // exit button
                (
                    Button,
                    button_node,
                    BackgroundColor(IDLE_BUTTON),
                    MenuButtonAction::Quit,
                    children![
                        (Text::new("Quit"), button_text_font, TextColor(TEXT_COLOR),),
                    ]
                ),
            ]
        )],
    ));
}
