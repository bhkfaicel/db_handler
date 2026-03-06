use std::collections::HashMap;

pub trait HasColumns {
    fn column_map() -> HashMap<String, String>;
}
