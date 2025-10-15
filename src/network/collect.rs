use serde::Serialize;
use std::collections::HashMap;
use zbus::Connection;
use zvariant::Value;

use super::nm_device::Nm;
use crate::error::AppError;

#[derive(Debug, Serialize)]
struct IfaceInfo {
    name: String,
    iface_type: String,
    state: String,
    ip4: Option<String>,
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
    for dev_path in nm.get_devices().await? {
        let device = nm.device(dev_path.as_str()).await?;
        let active = device.active().await?;
        if let Some(ipv4_config) = active.ipv4_conf().await {
            result.push(IfaceInfo {
                ip4: ipv4_config.ipv4().await,
                name: device.interface().await.unwrap_or_default(),
                iface_type: device
                    .device_type()
                    .await
                    .map(Nm::map_device_type)
                    .unwrap_or_default(),
                state: device
                    .state()
                    .await
                    .map(Nm::map_device_state)
                    .unwrap_or_default(),
            });
        }
    }
    let b = serde_json::to_vec(&result).map_err(AppError::Serialize)?;
    Ok(b)
}

// async fn extract_ipv4(conf: &zbus::Proxy<'_>) -> Option<String> {
//     conf.get_property::<Vec<HashMap<String, Value>>>("AddressData")
//         .await
//         .ok()?
//         .first()?
//         .get("address")
//         .and_then(|v| match v {
//             Value::Str(addr) => Some(addr.to_string()),
//             _ => None,
//         })
// }

#[allow(dead_code)]
async fn get_path(dev: &zbus::Proxy<'_>) -> Option<zbus::zvariant::OwnedObjectPath> {
    let active_conn_path = dev
        .get_property::<zbus::zvariant::OwnedObjectPath>("ActiveConnection")
        .await
        .ok()?;
    // if active_conn_path.as_str() != "/" {}
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
