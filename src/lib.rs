/// Serilization using the file system.
/// A serde backend which writes data using a directory tree, where leaf nodes contain values
///
/// # Example
/// ```
/// ```

mod de;
mod error;
mod ser;

pub use de::{from_fs, Deserializer};
pub use ser::{to_fs, Serializer};
