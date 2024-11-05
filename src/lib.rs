use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::{prelude::*, world::DeferredWorld};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

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

pub struct ReactionPlugin {
    fns: Vec<Arc<dyn Fn(&mut App) + Send + Sync>>,
}

impl ReactionPlugin {
    pub fn new() -> Self {
        let mut me = Self::empty();
        me.add_label(PostUpdate);
        me
    }

    pub fn empty() -> Self {
        Self { fns: Vec::new() }
    }

    pub fn add_label<L>(&mut self, label: L) -> &mut Self
    where
        L: ScheduleLabel + Clone,
    {
        let f = Arc::new(move |app: &mut App| {
            app.add_systems(label.clone(), react::<L>);
        });
        self.fns.push(f);
        self
    }
}

impl Default for ReactionPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for ReactionPlugin {
    fn build(&self, app: &mut App) {
        for f in &self.fns {
            f(app);
        }
    }
}

pub fn react<L: ScheduleLabel>(
    mut world: DeferredWorld,
    reaction_query: Query<(Entity, &Reaction<L>)>,
) {
    for (entity, reaction) in &reaction_query {
        reaction.run(world.reborrow(), entity);
    }
}
