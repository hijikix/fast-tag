pub mod s3;
pub mod azure;
pub mod gcs;
pub mod local;

pub use s3::S3StorageProvider;
pub use azure::AzureStorageProvider;
pub use gcs::GcsStorageProvider;
pub use local::LocalStorageProvider;