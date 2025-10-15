use serde::Serialize;
use std::collections::HashMap;
use zbus::Connection;
use zvariant::Value;

use super::nm_device::Nm;
use crate::{error::AppError, network::nm_device::DevType};

#[derive(Debug, Serialize)]
pub enum IfaceInfo {
    Ethernet(String),
    Wifi(String, String),
}

pub async fn collect_network_interface() -> Result<Vec<u8>, AppError> {
    let connection = match Connection::system().await {
        Ok(c) => c,
        Err(e) => {
            return Err(AppError::Failed(format!(
                r#"{{"error":"dbus connect failed: {e}"}}"#
            )));
        }
    };

    let nm = Nm::new(connection);

    let mut result = Vec::<IfaceInfo>::new();

    for device in nm.get_devices().await? {
        let dev_type = device.device_type().await?;
        match dev_type.into() {
            DevType::Ethernet => {
                if let Some(ipv4_config) = device.active().await?.ipv4_conf().await {
                    result.push(IfaceInfo::Ethernet(
                        ipv4_config.ipv4().await.unwrap_or_default(),
                    ));
                }
            }
            DevType::WiFi => {}
            _ => {}
        };
    }

    serde_json::to_vec(&result).map_err(AppError::Serialize)
}

#[allow(dead_code)]
async fn get_path(dev: &zbus::Proxy<'_>) -> Option<zbus::zvariant::OwnedObjectPath> {
    let active_conn_path = dev
        .get_property::<zbus::zvariant::OwnedObjectPath>("ActiveConnection")
        .await
        .ok()?;
    Some(active_conn_path)
}

#[allow(dead_code)]
async fn extract_ipv4(conf: Result<zbus::Proxy<'_>, zbus::Error>) -> Option<String> {
    let conf = conf.ok()?;
    let addrs = conf
        .get_property::<Vec<HashMap<String, Value>>>("AddressData")
        .await
        .ok()?;
    let addrmap = addrs.first()?;
    match addrmap.get("address") {
        Some(Value::Str(addr)) => Some(addr.to_string()),
        _ => None,
    }
}
