use crate::{ReactiveSystem, ReactiveSystemParam, Scope};
use bevy_ecs::{prelude::*, system::SystemParamItem, world::DeferredWorld};
use std::marker::PhantomData;

pub trait ReactiveSystemParamFunction<Marker> {
    type Param: ReactiveSystemParam;

    type In;

    type Out;

    fn run(
        &mut self,
        param: SystemParamItem<Self::Param>,
        input: Self::In,
        entity: Entity,
    ) -> Self::Out;
}

impl<Marker, F, T> ReactiveSystemParamFunction<Marker> for F
where
    F: SystemParamFunction<Marker, In = Scope<T>>,
    F::Param: ReactiveSystemParam,
{
    type Param = F::Param;

    type In = T;

    type Out = F::Out;

    fn run(
        &mut self,
        param: SystemParamItem<Self::Param>,
        input: Self::In,
        entity: Entity,
    ) -> Self::Out {
        SystemParamFunction::run(self, Scope { entity, input }, param)
    }
}

pub struct FunctionReactiveSystem<F, S, Marker> {
    pub(crate) f: F,
    pub(crate) state: Option<S>,
    pub(crate) _marker: PhantomData<Marker>,
}

impl<F, S, Marker> ReactiveSystem for FunctionReactiveSystem<F, S, Marker>
where
    F: ReactiveSystemParamFunction<Marker> + Send + Sync,
    F::Param: ReactiveSystemParam<State = S>,
    S: Send + Sync,
    Marker: Send + Sync,
{
    type In = F::In;
    type Out = F::Out;

    fn init(&mut self, world: &mut World) {
        self.state = Some(F::Param::init(world));
    }

    fn is_changed(&mut self, world: DeferredWorld) -> bool {
        F::Param::is_changed(world, self.state.as_mut().unwrap())
    }

    fn run(&mut self, input: Self::In, mut world: DeferredWorld, entity: Entity) -> Self::Out {
        // TODO check for overlapping params
        let mut world = world.reborrow();
        let params = unsafe { F::Param::get(&mut world, self.state.as_mut().unwrap()) };

        self.f.run(params, input, entity)
    }
}
