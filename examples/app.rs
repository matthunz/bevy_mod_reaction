use bevy::prelude::*;
use bevy_mod_reaction::{react, Reaction, Scope};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, react)
        .run();
}

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Damage(i32);

fn setup(mut commands: Commands) {
    commands.spawn((
        Health(100),
        Reaction::new(
            |scope: In<Scope>, mut commands: Commands, query: Query<&Health>| {
                let health = query.get(scope.entity).unwrap();
                commands.entity(scope.entity).insert(Damage(health.0 * 2));
            },
        ),
    ));

    commands.spawn(Reaction::new(|_: In<Scope>, query: Query<&Damage>| {
        for dmg in &query {
            dbg!(dmg.0);
        }
    }));
}
