use bevy::prelude::*;
use bevy_mod_reaction::{react, Reaction};

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
            |entity: In<Entity>, mut commands: Commands, query: Query<&Health>| {
                let health = query.get(*entity).unwrap();
                commands.entity(*entity).insert(Damage(health.0 * 2));
            },
        ),
    ));

    commands.spawn(Reaction::new(|_: In<Entity>, query: Query<&Damage>| {
        for dmg in &query {
            dbg!(dmg.0);
        }
    }));
}
