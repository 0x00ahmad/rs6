/// Proxy Error
#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    /// Hyper Error
    #[error("{0:?}")]
    HyperError(#[from] hyper::Error),

    /// Hyper Legacy Error
    #[error("{0:?}")]
    HyperLegacyError(#[from] hyper_util::client::legacy::Error),
}
