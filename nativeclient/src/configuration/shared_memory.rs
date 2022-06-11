use serde::Serialize;

#[derive(Serialize, Default)]
pub struct SharedMemory {
    heap_size: Option<i32>,
    dependency_table_power: Option<i32>,
    hash_table_power: Option<i32>,
}
