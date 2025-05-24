use bevy::ecs::relationship::RelatedSpawner;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy::{color::palettes::tailwind::*, ecs::spawn::SpawnWith};
use widgets::button::{ButtonBackground, LabelButton};

pub mod widgets;
pub mod world_space;

pub(super) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            world_space::WorldSpaceUiPlugin,
            widgets::WidgetsPlugin,
        ));

        app.add_systems(Startup, setup_menu);

        app.init_state::<Screen>();
    }
}

fn setup_menu(mut commands: Commands) {
    commands.spawn((
        StateScoped(Screen::Menu),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::VMin(10.0)),
            justify_content: JustifyContent::End,
            align_items: AlignItems::End,
            ..default()
        },
        FocusPolicy::Pass,
        Pickable::IGNORE,
        Children::spawn(Spawn((
            Node {
                padding: UiRect::all(Val::VMin(6.0)),
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.2)),
            BorderRadius::all(Val::VMin(4.0)),
            Children::spawn((
                Spawn((
                    Text::new("Recipe"),
                    TextFont::from_font_size(64.0),
                    TextColor(ORANGE_600.into()),
                    TextShadow::default(),
                )),
                SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
                    parent
                        .spawn(
                            LabelButton::new("Play!")
                                .with_bacground(
                                    ButtonBackground::new(SKY_500),
                                )
                                .build(),
                        )
                        .observe(play_on_click);

                    // Only add exit button for non-web game.
                    #[cfg(not(target_arch = "wasm32"))]
                    parent
                        .spawn(
                            LabelButton::new("Exit..")
                                .with_bacground(
                                    ButtonBackground::new(RED_500),
                                )
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
    screen.set(Screen::LevelSelection);
}

#[cfg(not(target_arch = "wasm32"))]
fn exit_on_click(
    _: Trigger<Pointer<Click>>,
    mut exit: EventWriter<AppExit>,
) {
    exit.write(AppExit::Success);
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
#[states(scoped_entities)]
pub enum Screen {
    #[default]
    Menu,
    LevelSelection,
    _EnterLevel, // TODO: Create substates for levels (1, 2, 3, ...).
}
