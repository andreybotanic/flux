mod mod_id;
mod namespaced_id;
mod prototype_id;
mod validation;

pub use mod_id::ModId;
pub use namespaced_id::NamespacedId;
pub use prototype_id::PrototypeId;

#[cfg(test)]
mod tests;
