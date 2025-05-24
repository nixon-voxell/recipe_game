use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub(super) struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_kbm)
            .add_observer(setup_gamepads);
    }
}

/// Setup only 1 [`InputMap`] for keyboard actions.
fn setup_kbm(mut commands: Commands) {
    let entity = commands.spawn(PlayerAction::new_kbm()).id();
    info!("Setup `PlayerAction` input map for keyboard {entity}.");
}

/// Create a [`InputMap`] for every connected gamepads.
fn setup_gamepads(
    trigger: Trigger<OnAdd, Gamepad>,
    mut commands: Commands,
) {
    let entity = trigger.target();
    commands
        .entity(entity)
        .insert(PlayerAction::new_gamepad().with_gamepad(entity));

    info!("Setup `PlayerAction` input map for gamepad {entity}.");
}

#[derive(
    Actionlike, Reflect, PartialEq, Eq, Clone, Copy, Hash, Debug,
)]
pub enum PlayerAction {
    #[actionlike(DualAxis)]
    Move,
    #[actionlike(DualAxis)]
    Aim,
    Jump,
    Interact,
    Attack,
}

impl PlayerAction {
    /// Create a new [`InputMap`] for gamepads.
    pub fn new_gamepad() -> InputMap<Self> {
        InputMap::default()
            // Gamepad input bindings.
            .with_dual_axis(Self::Move, GamepadStick::LEFT)
            .with_dual_axis(Self::Aim, GamepadStick::RIGHT)
            .with(Self::Jump, GamepadButton::South)
            .with(Self::Interact, GamepadButton::West)
            .with(Self::Attack, GamepadButton::RightTrigger2)
    }

    /// Create a new [`InputMap`] for keyboard and mouse.
    pub fn new_kbm() -> InputMap<Self> {
        InputMap::default()
            // KbM input bindings.
            .with_dual_axis(Self::Move, VirtualDPad::wasd())
            .with_dual_axis(Self::Aim, MouseMove::default())
            .with(Self::Jump, KeyCode::Space)
            .with(Self::Interact, KeyCode::KeyE)
            .with(Self::Attack, MouseButton::Left)
    }
}
