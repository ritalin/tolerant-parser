use engine_wasi::interface;
use engine_wasi::resource;

pub struct SqliteEngineComponent;

impl interface::Guest for SqliteEngineComponent {
    type Engine = engine_wasi::resource::EngineImpl;

    fn create() -> Result<interface::Engine, interface::EngineError> {
        match sqlite_engine::create() {
            Ok(engine) => Ok(interface::Engine::new(resource::EngineImpl::new(engine))),
            Err(err) => Err(interface::EngineError::CreateFailed(err.to_string())),
        }
    }
}

engine_wasi::export!(SqliteEngineComponent);