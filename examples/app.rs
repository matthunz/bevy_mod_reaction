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
    commands.spawn((
        X(0),
        Reaction::new(|entity: In<Entity>, query: Query<&X>| {
            dbg!(query.get(*entity).unwrap().0);
        }),
    ));
}
