use wasmtime::{component::{Component, Linker}, Engine, Store};
use wasmtime_wasi::{p2::{WasiCtx, WasiCtxBuilder}, ResourceTable};

pub struct Runtime {
    engine: Engine,
    linker: Linker<RuntimeState>,
}

impl Runtime {
    pub fn create() -> Result<Self, anyhow::Error> {
        let engine = Engine::default();
        let mut linker = Linker::<RuntimeState>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync::<RuntimeState>(&mut linker)?;

        Ok(Self { engine, linker })
    }

    pub fn new_store(&self) -> Store<RuntimeState> {
        let context = RuntimeState::new(WasiCtxBuilder::new().build());
        Store::new(&self.engine, context)
    }

    pub fn prebuild(&self, wasm_path: &std::path::Path) -> Result<Vec<u8>, anyhow::Error> {
        let Ok(component) = Component::from_file(&self.engine, wasm_path)
        else {
            anyhow::bail!("wasm is not found");
        };
        component.serialize()
    }

    pub fn instanciate<World: InstanciateWorld<State = RuntimeState>>(&self, cwasm_path: &std::path::Path) -> Result<(World::World, Store<RuntimeState>), anyhow::Error> {
        let bynaly = std::fs::read(cwasm_path)?;
        let component = unsafe { Component::deserialize(&self.engine, bynaly)? };
        
        let mut store = self.new_store();
        let bindings = World::instantiate_component(&mut store, &component, &self.linker)?;

        Ok((bindings, store))
    }

}

pub struct RuntimeState {
    ctx: WasiCtx,
    resource_table: ResourceTable,
}

impl RuntimeState {
    pub fn new(ctx: WasiCtx) -> Self {
        Self {
            ctx,
            resource_table: ResourceTable::new(),
        }
    }
}

impl wasmtime_wasi::p2::WasiView for RuntimeState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
}

impl wasmtime_wasi::p2::IoView for RuntimeState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

pub trait InstanciateWorld {
    type World: 'static;
    type State: 'static;
    fn instantiate_component(store: impl wasmtime::AsContextMut<Data = Self::State>, component: &Component, linker: &Linker<Self::State>) -> Result<Self::World, anyhow::Error>;
}