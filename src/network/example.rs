use serde::Serialize;
use zbus::{Connection, Proxy};
use zvariant::Value;

#[derive(Debug, Serialize)]
struct IfaceInfo {
    name: String,
    iface_type: String,
    state: String,
    ip4: Option<String>,
    ssid: Option<String>,
    strength: Option<u8>,
}

pub async fn collect_netifs_json() -> String {
    let connection = match Connection::system().await {
        Ok(c) => c,
        Err(e) => return format!(r#"{{"error":"dbus connect failed: {e}"}}"#),
    };

    let nm = Proxy::new(
        &connection,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .await;

    let nm = match nm {
        Ok(p) => p,
        Err(e) => return format!(r#"{{"error":"nm proxy failed: {e}"}}"#),
    };

    let devices: Vec<zbus::zvariant::OwnedObjectPath> = match nm.call("GetDevices", &()).await {
        Ok(v) => v,
        Err(e) => return format!(r#"{{"error":"GetDevices failed: {e}"}}"#),
    };

    let mut results = Vec::new();

    for dev_path in devices {
        let dev = Proxy::new(
            &connection,
            "org.freedesktop.NetworkManager",
            dev_path.as_str(),
            "org.freedesktop.NetworkManager.Device",
        )
        .await
        .unwrap();

        let iface: String = dev.get_property("Interface").await.unwrap_or_default();
        let iface_type: u32 = dev.get_property("DeviceType").await.unwrap_or(0);
        let state: u32 = dev.get_property("State").await.unwrap_or(0);

        // Переводим тип и состояние в текст
        let iface_type_str = match iface_type {
            1 => "Ethernet",
            2 => "WiFi",
            5 => "Bluetooth",
            _ => "Other",
        }
        .to_string();

        let state_str = match state {
            100 => "Activated",
            50 => "Disconnected",
            _ => "Unknown",
        }
        .to_string();

        let mut ip4 = None;
        let mut ssid = None;
        let mut strength = None;

        // Если это Wi-Fi, достанем детали
        if iface_type == 2 {
            let wifi = Proxy::new(
                &connection,
                "org.freedesktop.NetworkManager",
                dev_path.as_str(),
                "org.freedesktop.NetworkManager.Device.Wireless",
            )
            .await;

            if let Ok(wifi) = wifi {
                if let Ok(ap_path) = wifi
                    .get_property::<zbus::zvariant::OwnedObjectPath>("ActiveAccessPoint")
                    .await
                {
                    let ap = Proxy::new(
                        &connection,
                        "org.freedesktop.NetworkManager",
                        ap_path.as_str(),
                        "org.freedesktop.NetworkManager.AccessPoint",
                    )
                    .await;

                    if let Ok(ap) = ap {
                        if let Ok(v) = ap.get_property::<Vec<u8>>("Ssid").await {
                            ssid = Some(String::from_utf8_lossy(&v).to_string());
                        }
                        strength = ap.get_property::<u8>("Strength").await.ok();
                    }
                }
            }
        }

        // IPv4 адреса можно достать через ActiveConnection → Ip4Config
        if let Ok(active_conn_path) = dev
            .get_property::<zbus::zvariant::OwnedObjectPath>("ActiveConnection")
            .await
        {
            if active_conn_path.as_str() != "/" {
                let active = Proxy::new(
                    &connection,
                    "org.freedesktop.NetworkManager",
                    active_conn_path.as_str(),
                    "org.freedesktop.NetworkManager.Connection.Active",
                )
                .await;

                if let Ok(active) = active {
                    if let Ok(ip4conf_path) = active
                        .get_property::<zbus::zvariant::OwnedObjectPath>("Ip4Config")
                        .await
                    {
                        let ip4conf = Proxy::new(
                            &connection,
                            "org.freedesktop.NetworkManager",
                            ip4conf_path.as_str(),
                            "org.freedesktop.NetworkManager.IP4Config",
                        )
                        .await;

                        if let Ok(ip4conf) = ip4conf {
                            if let Ok(addrs) = ip4conf
                                .get_property::<Vec<HashMap<String, Value>>>("AddressData")
                                .await
                            {
                                if let Some(addrmap) = addrs.first() {
                                    if let Some(Value::Str(addr)) = addrmap.get("address") {
                                        ip4 = Some(addr.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        results.push(IfaceInfo {
            name: iface,
            iface_type: iface_type_str,
            state: state_str,
            ip4,
            ssid,
            strength,
        });
    }

    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}
