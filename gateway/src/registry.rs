use crate::config::Function;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

lazy_static! {
    pub static ref SERVICE_CONFIG_MAP: Mutex<HashMap<String, Function>> = Mutex::new(HashMap::new());
}

pub struct Registry {
    pub service_registry: Mutex<HashMap<String, (IpAddr, u16)>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            service_registry: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_service_endpoint(&self, service_name: &str) -> Option<(IpAddr, u16)> {
        let registry = self.service_registry.lock().unwrap();
        registry.get(service_name).copied()
    }

    pub fn insert_service(&self, service_name: &str, endpoint: (IpAddr, u16)) {
        let mut registry = self.service_registry.lock().unwrap();
        registry.insert(service_name.to_string(), endpoint);
    }

    pub fn check_service_config(service_name: &str) -> bool {
        let functions = SERVICE_CONFIG_MAP.lock().unwrap();
        functions.contains_key(service_name)
    }
    pub fn get_service_from_config(service_name:&str)->Option<Function>{
        let functions = SERVICE_CONFIG_MAP.lock().unwrap();
        functions.get(service_name).cloned()

    }
}
