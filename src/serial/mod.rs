pub mod errors;
mod port;
mod traits;
mod wrapper;
mod mock_serial;
mod buffer;

pub use traits::Connection;
pub use errors::Error;
pub use port::new;
