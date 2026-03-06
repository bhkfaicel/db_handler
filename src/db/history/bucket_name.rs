pub enum BucketName {
    /// sensor
    Sensor,
}

impl std::fmt::Display for BucketName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BucketName::Sensor => write!(f, "test_bucket"),
        }
    }
}
