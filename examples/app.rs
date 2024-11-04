use bevy::prelude::*;
use bevy_mod_reaction::{react, Reaction};

fn main() {
    App::new()
        .add_systems(Startup, setup)
        .add_systems(Update, react)
        .run();
}

#[derive(Component)]
struct X(i32);

fn setup(mut commands: Commands) {
    commands.spawn(X(0));

    commands.spawn(Reaction::new(|query: Query<&X>| {
        for x in &query {
            dbg!(x.0);
        }
    }));
}
