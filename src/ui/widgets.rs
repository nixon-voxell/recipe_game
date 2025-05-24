use bevy::prelude::*;

pub mod button;

pub(super) struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(button::ButtonPlugin);
    }
}
