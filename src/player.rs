use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;

use crate::ui::world_space::WorldUi;

pub(super) struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PlayerState>()
            .init_resource::<PlayerPossession>()
            .add_observer(setup_name_ui_for_player)
            .add_observer(setup_player_tag)
            .add_systems(
                OnEnter(PlayerState::Possessing),
                setup_possessing_ui,
            )
            .add_systems(
                Update,
                process_posessing_inputs
                    .run_if(in_state(PlayerState::Possessing)),
            );

        app.register_type::<PlayerType>();
    }
}

fn process_posessing_inputs(
    q_gamepads: Query<(&Gamepad, Entity)>,
    kbd_inputs: Res<ButtonInput<KeyCode>>,
    mut player_possession: ResMut<PlayerPossession>,
) {
    if kbd_inputs.just_pressed(KeyCode::KeyA) {
        // Remove previous possession if any.
        if player_possession.player_b == Some(Possession::Keyboard) {
            player_possession.player_b = None;
        }

        // Assign current possesion.
        player_possession.player_a = Some(Possession::Keyboard);
    }

    if kbd_inputs.just_pressed(KeyCode::KeyD) {
        // Remove previous possession if any.
        if player_possession.player_a == Some(Possession::Keyboard) {
            player_possession.player_a = None;
        }

        // Assign current possesion.
        player_possession.player_b = Some(Possession::Keyboard);
    }

    // Handle cancelation.
    if kbd_inputs.just_pressed(KeyCode::Escape) {
        if player_possession.player_a == Some(Possession::Keyboard) {
            player_possession.player_a = None;
        }

        if player_possession.player_b == Some(Possession::Keyboard) {
            player_possession.player_b = None;
        }
    }

    for (gamepad, entity) in q_gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::DPadLeft) {
            // Remove previous possession if any.
            if player_possession.player_b
                == Some(Possession::Gamepad(entity))
            {
                player_possession.player_b = None;
            }

            // Assign current possesion.
            player_possession.player_a =
                Some(Possession::Gamepad(entity));
        }

        if gamepad.just_pressed(GamepadButton::DPadRight) {
            // Remove previous possession if any.
            if player_possession.player_a
                == Some(Possession::Gamepad(entity))
            {
                player_possession.player_a = None;
            }

            // Assign current possesion.
            player_possession.player_b =
                Some(Possession::Gamepad(entity));
        }

        // Handle cancelation.
        if gamepad.just_pressed(GamepadButton::East) {
            if player_possession.player_a
                == Some(Possession::Gamepad(entity))
            {
                player_possession.player_a = None;
            }

            if player_possession.player_b
                == Some(Possession::Gamepad(entity))
            {
                player_possession.player_b = None;
            }
        }
    }
}

fn setup_possessing_ui(mut commands: Commands) {
    let instruction_ui_node = Node {
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::all(Val::VMin(6.0)),
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        ..default()
    };

    let instruction_content_ui = children![
        (
            instruction_ui_node.clone(),
            children![(
                Text::new(
                    "Press:\nA (keyboard) / DPadLeft (controller)\n...to possess Player A",
                ),
                // TextLayout::new_with_justify(JustifyText::Center),
            )],
        ),
        // Separation line.
        (
            Node {
                width: Val::Px(10.0),
                height: Val::Percent(80.0),
                ..default()
            },
            BackgroundColor(GRAY_200.into()),
        ),
        (
            instruction_ui_node,
            children![
                Text::new(
                    "Press:\nD (keyboard) / DPadRight (controller)\n...to possess Player B",
                ),
                // TextLayout::new_with_justify(JustifyText::Center),
            ],
        ),
    ];

    let instruction_ui = children![
        (
            Text::new(
                "Press Esc (keyboard) | B (controller) to cancel."
            ),
            TextLayout::new_with_justify(JustifyText::Center),
        ),
        (
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_grow: 1.0,
                ..default()
            },
            instruction_content_ui,
        )
    ];

    commands.spawn((
        StateScoped(PlayerState::Possessing),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::VMin(10.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        // Should be on top of all other uis.
        GlobalZIndex(10),
        children![(
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::VMin(4.0)),
                flex_grow: 1.0,
                ..default()
            },
            BackgroundColor(ZINC_950.with_alpha(0.8).into()),
            BorderRadius::all(Val::VMin(4.0)),
            BoxShadow::new(
                ZINC_950.with_alpha(0.9).into(),
                Val::Auto,
                Val::Auto,
                Val::Px(20.0),
                Val::Px(16.0),
            ),
            instruction_ui,
        )],
    ));
}

/// Setup world space name ui for players.
fn setup_name_ui_for_player(
    trigger: Trigger<OnAdd, PlayerType>,
    mut commands: Commands,
    q_players: Query<&PlayerType>,
) -> Result {
    let entity = trigger.target();

    let player_type = q_players.get(entity)?;

    let world_ui =
        WorldUi::new(entity).with_world_offset(Vec3::Y * 0.5);
    let ui_bundle = move |name: &str| {
        (
            world_ui,
            Node {
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BorderRadius::all(Val::Px(8.0)),
            BackgroundColor(ZINC_900.with_alpha(0.5).into()),
            BoxShadow::new(
                ZINC_900.into(),
                Val::Px(4.0),
                Val::Px(4.0),
                Val::Px(14.0),
                Val::Px(12.0),
            ),
            Children::spawn(Spawn((
                Text::new(name),
                TextLayout::new_with_justify(JustifyText::Center),
            ))),
        )
    };

    match player_type {
        PlayerType::A => {
            commands.spawn(ui_bundle("Player A"));
        }
        PlayerType::B => {
            commands.spawn(ui_bundle("Player B"));
        }
    }

    Ok(())
}

/// Setup player tag: [`PlayerA`] and [`PlayerB`]
/// based on [`PlayerType`].
fn setup_player_tag(
    trigger: Trigger<OnAdd, PlayerType>,
    mut commands: Commands,
    q_players: Query<&PlayerType>,
) -> Result {
    let entity = trigger.target();

    let player_type = q_players.get(entity)?;

    match player_type {
        PlayerType::A => {
            commands.entity(entity).insert(PlayerA);
        }
        PlayerType::B => {
            commands.entity(entity).insert(PlayerB);
        }
    }

    Ok(())
}

// TODO: Rename these to the character's name!

#[derive(Component, Reflect, Debug)]
#[reflect(Component)]
pub enum PlayerType {
    A,
    B,
}

/// A unique component tag for player A.
#[derive(Component, Debug)]
pub struct PlayerA;

/// A unique component tag for player B.
#[derive(Component, Debug)]
pub struct PlayerB;

#[derive(Clone, Eq, PartialEq, Debug, Hash, Default, States)]
#[states(scoped_entities)]
pub enum PlayerState {
    #[default]
    Possessing,
    _Possessed,
}

/// The currently possession state of the players.
#[derive(Resource, Default, Debug)]
pub struct PlayerPossession {
    pub player_a: Option<Possession>,
    pub player_b: Option<Possession>,
}

/// Possesion state, can be keyboard or a specific gamepad.
#[derive(Debug, PartialEq, Eq)]
pub enum Possession {
    Keyboard,
    Gamepad(Entity),
}
