use bevy_ecs::{
    prelude::*,
    query::{QueryData, QueryFilter},
    system::SystemState,
    world::DeferredWorld,
};
use std::mem;

pub trait ReactiveQueryData<F: QueryFilter>: QueryData + Sized {
    type State: Send + Sync + 'static;

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State;

    fn is_changed(world: DeferredWorld, state: &mut <Self as ReactiveQueryData<F>>::State) -> bool;

    fn is_changed_with_entity(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
        entity: Entity,
    ) -> bool;

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData<F>>::State,
    ) -> Query<'w, 's, Self, F>;
}

impl<F, T> ReactiveQueryData<F> for &T
where
    F: QueryFilter + 'static,
    T: Component,
{
    type State = SystemState<(
        Query<'static, 'static, (), (Changed<T>, F)>,
        Query<'static, 'static, &'static T, F>,
    )>;

    fn init(world: &mut World) -> <Self as ReactiveQueryData<F>>::State {
        SystemState::new(world)
    }

    fn is_changed<'w>(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
    ) -> bool {
        !state.get(&world).0.is_empty()
    }

    fn is_changed_with_entity(
        world: DeferredWorld,
        state: &mut <Self as ReactiveQueryData<F>>::State,
        entity: Entity,
    ) -> bool {
        state.get(&world).0.get(entity).is_ok()
    }

    fn get<'w, 's>(
        world: &'w mut DeferredWorld<'w>,
        state: &'s mut <Self as ReactiveQueryData<F>>::State,
    ) -> Query<'w, 's, Self, F> {
        // TODO verify safety
        unsafe { mem::transmute(state.get(world).1) }
    }
}
