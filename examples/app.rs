use bevy::prelude::*;
use bevy_mod_reaction::{Reaction, ReactionPlugin, ReactiveQuery, Scope};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ReactionPlugin::new()))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Clone, Copy, Component)]
struct Health(i32);

#[derive(Component)]
struct Damage(i32);

#[allow(unused)]
#[derive(Component)]
struct Armor(i32);

fn setup(mut commands: Commands) {
    // Coarse-grained reactivity:
    // This reaction will only run when a `Damage` component changes.
    commands.spawn(Reaction::new(|_: In<Scope>, query: Query<&Damage>| {
        for dmg in &query {
            dbg!(dmg.0);
        }
    }));

    // Fine-grained reactivity:
    // This reaction will only run when a tracked `Health` component changes.
    commands.spawn((
        Health(0),
        Reaction::switch(
            |scope: In<Scope>, mut query: ReactiveQuery<&Health>| {
                let health = query.get(scope.entity).unwrap();
                health.0 == 0
            },
            || Armor(50),
            || Damage(100),
        ),
    ));

    // Reactions can also subscribe to multiple targets.
    let a = commands.spawn(Health(100)).id();
    let b = commands.spawn(Health(100)).id();

    let mut reaction = Reaction::derive(|scope: In<Scope>, query: Query<&Health>| {
        let health = query.get(scope.entity).unwrap();
        Damage(health.0 * 2)
    });
    reaction.add_target(a);
    reaction.add_target(b);
    commands.spawn(reaction);

    // Reactions can also be created from iterators.
    commands.spawn(Reaction::from_iter(
        |_: In<Scope>, query: Query<&Health>| {
            query.iter().map(|health| *health).collect::<Vec<_>>()
        },
    ));
}
