use bevy::prelude::*;
use bevy_mod_reaction::{react, Reaction};

fn main() {
    App::new()
        .insert_resource(X(0))
        .add_systems(Startup, setup)
        .add_systems(Update, react)
        .run();
}

#[derive(Resource)]
struct X(i32);

fn setup(mut commands: Commands) {
    commands.spawn(Reaction::new(|x: Res<X>| {
        dbg!(x.0);
    }));
}
