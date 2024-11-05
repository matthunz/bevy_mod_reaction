use bevy::prelude::*;
use bevy_mod_reaction::{react, Reaction, ReactiveQuery, Scope};

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

#[allow(unused)]
#[derive(Component)]
struct Armor(i32);

fn setup(mut commands: Commands) {
    // Coarse-grained reactivity:
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
}
