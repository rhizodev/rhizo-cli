use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub encodings: Vec<String>,
    pub arguments: Vec<Argument>,
    pub route: String,
    pub cacheable: bool,
    pub cache_ttl_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Argument {
    pub name: String,
    pub argument_type: String,
}
