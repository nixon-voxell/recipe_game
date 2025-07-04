use bevy::asset::load_internal_binary_asset;
use bevy::ecs::spawn::SpawnWith;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use widgets::button::{ButtonBackground, LabelButton};

use crate::asset_pipeline::{AssetState, SceneAssetsLoader};

mod game_over_ui;
mod health_bar_ui;
mod inventory_ui;
mod player_mark_ui;
mod wave_countdown_ui;
pub mod widgets;
pub mod world_space;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            world_space::WorldSpaceUiPlugin,
            widgets::WidgetsPlugin,
            inventory_ui::InventoryUiPlugin,
            health_bar_ui::HealthBarUiPlugin,
            player_mark_ui::PlayerMarkUiPlugin,
            game_over_ui::GameOverUiPlugin,
            wave_countdown_ui::WaveCountdownUiPlugin,
        ));

        app.add_sub_state::<Screen>()
            .add_systems(
                OnEnter(Screen::Menu),
                (
                    setup_menu,
                    load_default_scene,
                    set_cursor_grab_mode(CursorGrabMode::None),
                ),
            )
            .add_systems(
                OnEnter(Screen::EnterLevel),
                (
                    load_level1,
                    set_cursor_grab_mode(CursorGrabMode::Locked),
                ),
            )
            .add_systems(
                OnEnter(Screen::GameOver),
                set_cursor_grab_mode(CursorGrabMode::None),
            );

        load_internal_binary_asset!(
            app,
            TextFont::default().font,
            "../assets/fonts/Cherry_Bomb_One/CherryBombOne-Regular.ttf",
            |bytes: &[u8], _path: String| {
                Font::try_from_bytes(bytes.to_vec()).unwrap()
            }
        );
    }
}

fn set_cursor_grab_mode(
    grab_mode: CursorGrabMode,
) -> impl Fn(Query<'_, '_, &mut Window, With<PrimaryWindow>>) {
    move |mut q_windows: Query<&mut Window, With<PrimaryWindow>>| {
        let Ok(mut window) = q_windows.single_mut() else {
            error!("No primary window!");
            return;
        };

        window.cursor_options.grab_mode = grab_mode;
        window.cursor_options.visible = match grab_mode {
            CursorGrabMode::None => true,
            CursorGrabMode::Confined => true,
            CursorGrabMode::Locked => false,
        };
    }
}

fn load_default_scene(mut scenes: SceneAssetsLoader) -> Result {
    scenes.load_default_scene()
}

fn load_level1(mut scenes: SceneAssetsLoader) -> Result {
    scenes.load_level1()
}

fn setup_menu(mut commands: Commands) {
    const FONT_SIZE: f32 = 30.0;

    let bg_color = Srgba::hex("BFB190").unwrap().with_alpha(0.4);
    let font_color = Srgba::hex("342C24").unwrap();
    let play_color = Srgba::hex("FFDE59").unwrap().with_alpha(0.45);
    let exit_color = Srgba::hex("856850").unwrap().with_alpha(0.45);

    commands.spawn((
        StateScoped(Screen::Menu),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(40.0)),
            justify_content: JustifyContent::End,
            align_items: AlignItems::End,
            ..default()
        },
        FocusPolicy::Pass,
        Pickable::IGNORE,
        Children::spawn(Spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_grow: 0.0,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(bg_color.into()),
            BorderRadius::all(Val::Px(40.0)),
            Children::spawn((
                Spawn((
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    Text::new("Bunguette"),
                    TextFont::from_font_size(FONT_SIZE * 1.5),
                    TextColor(font_color.into()),
                )),
                SpawnWith(move |parent: &mut ChildSpawner| {
                    parent
                        .spawn(
                            LabelButton::new("Play")
                                .with_background(
                                    ButtonBackground::new(play_color),
                                )
                                .with_text_color(font_color)
                                .with_font_size(FONT_SIZE)
                                .build(),
                        )
                        .observe(play_on_click);

                    // Only add exit button for non-web game.
                    #[cfg(not(target_arch = "wasm32"))]
                    parent
                        .spawn(
                            LabelButton::new("Exit")
                                .with_background(
                                    ButtonBackground::new(exit_color),
                                )
                                .with_text_color(font_color)
                                .with_font_size(FONT_SIZE)
                                .build(),
                        )
                        .observe(exit_on_click);
                }),
            )),
        ))),
    ));
}

fn play_on_click(
    _: Trigger<Pointer<Click>>,
    mut screen: ResMut<NextState<Screen>>,
) {
    // screen.set(Screen::LevelSelection);
    screen.set(Screen::EnterLevel);
}

#[cfg(not(target_arch = "wasm32"))]
fn exit_on_click(
    _: Trigger<Pointer<Click>>,
    mut exit: EventWriter<AppExit>,
) {
    exit.write(AppExit::Success);
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, SubStates)]
#[states(scoped_entities)]
#[source(AssetState = AssetState::Loaded)]
pub enum Screen {
    #[default]
    Menu,
    // LevelSelection,
    EnterLevel, // TODO: Create substates for levels (1, 2, 3, ...).
    GameOver,
}
