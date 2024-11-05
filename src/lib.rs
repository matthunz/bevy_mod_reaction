use bevy_ecs::{prelude::*, world::DeferredWorld};
use std::ops::{Deref, DerefMut};

mod into_system;
pub use self::into_system::{IntoReactiveSystem, Map};

mod query_data;
pub use self::query_data::ReactiveQueryData;

mod reaction;
pub use self::reaction::Reaction;

mod system;
pub use self::system::ReactiveSystem;

mod system_fn;
pub use self::system_fn::{FunctionReactiveSystem, ReactiveSystemParamFunction};

mod system_param;
pub use self::system_param::{ReactiveQuery, ReactiveQueryState, ReactiveSystemParam};

pub struct Scope<T = ()> {
    pub entity: Entity,
    pub input: T,
}

impl<T> Deref for Scope<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.input
    }
}

impl<T> DerefMut for Scope<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.input
    }
}

pub fn react(mut world: DeferredWorld, reaction_query: Query<(Entity, &Reaction)>) {
    for (entity, reaction) in &reaction_query {
        reaction.run(world.reborrow(), entity);
    }
}
