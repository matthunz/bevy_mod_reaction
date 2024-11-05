use bevy_ecs::{
    entity::Entity,
    world::{DeferredWorld, World},
};

pub trait ReactiveSystem: Send + Sync {
    type In;

    type Out;

    fn init(&mut self, world: &mut World);

    fn is_changed(&mut self, world: DeferredWorld) -> bool;

    fn run(&mut self, input: Self::In, world: DeferredWorld, entity: Entity) -> Self::Out;
}
