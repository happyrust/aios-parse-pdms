#[cfg(not(target_arch = "wasm32"))]
pub mod increment_modify;

pub enum NewDataState {
    Modify,
    Increase,
    Delete
}