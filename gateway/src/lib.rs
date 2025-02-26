use serde::Deserialize;
use config::{Config, File};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Provider {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub gateway: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Function {
    pub lang: String,
    pub handler: String,
    pub image: String,
    #[serde(default)]
    pub memory: String,
    #[serde(default)]
    pub environment: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpenFaaSConfig {
    pub provider: Provider,
    pub functions: HashMap<String, Function>,
}

pub fn load_config(file_path: &str) -> Result<OpenFaaSConfig, config::ConfigError> {
    let config = Config::builder()
        .set_default("provider.name", "openfaas")?
        .set_default("provider.gateway", "http://127.0.0.1:8090")?
        .set_default("provider.memory", "128M")?
        .set_default("provider.environment", "")?
        // 添加 YAML 文件作为配置源
        .add_source(File::with_name(file_path))
        .build()?;
    // 反序列化为配置结构体
    config.try_deserialize()
}