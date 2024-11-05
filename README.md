# bevy_mod_reaction

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/matthunz/bevy_mod_reaction)
[![Crates.io](https://img.shields.io/crates/v/bevy_mod_reaction.svg)](https://crates.io/crates/bevy_mod_reaction)
[![Downloads](https://img.shields.io/crates/d/bevy_mod_reaction.svg)](https://crates.io/crates/bevy_mod_reaction)
[![Docs](https://docs.rs/bevy_mod_reaction/badge.svg)](https://docs.rs/bevy_mod_reaction/latest/bevy_mod_reaction/)
[![CI](https://github.com/matthunz/bevy_mod_reaction/workflows/CI/badge.svg)](https://github.com/matthunz/bevy_mod_reaction/actions)


Reactive components for Bevy.

A `Reaction` is a component around a `ReactiveSystem`, which runs every time its parameters have changed. Bevy's built-in change detection mechanisms are used to efficiently react to changes in state.
```rs
// Coarse-grained reactivity:
// This reaction will only run when a`Damage` component changes.
commands.spawn(Reaction::new(|_: In<Scope>, query: Query<&Damage>| {
    for dmg in &query {
        dbg!(dmg.0);
    }
}));
```

For coarse-grained reactivity `ReactiveQuery` tracks the entities read and only re-runs the current system if those values have changed. Bundles of components can also be derived:
```rs
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
```

Switch statements are also supported, with more primitives coming soon
```rs
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
```
