use std::collections::HashMap;
use zbus::{Connection, Proxy, zvariant::OwnedObjectPath};
use zvariant::Value;

use super::traits::NmChildObject;
use crate::{error::AppError, impl_nm_child};
pub enum NmIface {}

impl NmIface {
    pub const MAIN_PATH: &'static str = "/org/freedesktop/NetworkManager";
    // --- Основные интерфейсы NetworkManager ---
    pub const CORE: &'static str = "org.freedesktop.NetworkManager";
    pub const SETTINGS: &'static str = "org.freedesktop.NetworkManager.Settings";
    pub const ACTIVE_CONNECTION: &'static str = "org.freedesktop.NetworkManager.Connection.Active";

    // --- Устройства ---
    pub const DEVICE: &'static str = "org.freedesktop.NetworkManager.Device";
    pub const DEVICE_WIRELESS: &'static str = "org.freedesktop.NetworkManager.Device.Wireless";
    pub const DEVICE_WIRED: &'static str = "org.freedesktop.NetworkManager.Device.Wired";

    // --- IP-конфигурации ---
    pub const IP4_CONFIG: &'static str = "org.freedesktop.NetworkManager.IP4Config";
    pub const IP6_CONFIG: &'static str = "org.freedesktop.NetworkManager.IP6Config";

    // --- Свойства / сигналы ---
    pub const PROPERTIES: &'static str = "org.freedesktop.DBus.Properties";
    pub const ACCESSPOINT: &'static str = "org.freedesktop.NetworkManager.AccessPoint";
}

pub struct Nm {
    pub conn: Connection,
}
pub struct NmDevice<'a> {
    object_path: OwnedObjectPath,
    device: &'a Nm,
}

impl_nm_child!(NmDevice, NmIface::DEVICE);

impl<'a> From<(OwnedObjectPath, &'a Nm)> for NmDevice<'a> {
    fn from((object_path, device): (OwnedObjectPath, &'a Nm)) -> Self {
        Self {
            object_path,
            device,
        }
    }
}

impl Nm {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub fn map_device_state(state: u32) -> String {
        match state {
            100 => "Activated",
            50 => "Disconnected",
            _ => "Unknown",
        }
        .to_string()
    }

    pub async fn proxy<'a>(
        &self,
        path: &'a str,
        interface: &'static str,
    ) -> Result<Proxy<'a>, AppError> {
        Proxy::new(&self.conn, NmIface::CORE, path, interface)
            .await
            .map_err(AppError::ZBusError)
    }

    pub async fn main<'a>(&self) -> Result<Proxy<'a>, AppError> {
        self.proxy(NmIface::MAIN_PATH, NmIface::CORE).await
    }

    pub async fn get_devices<'a>(&'a self) -> Result<Vec<NmDevice<'a>>, AppError> {
        let devs_path: Vec<OwnedObjectPath> = self
            .main()
            .await?
            .call("GetDevices", &())
            .await
            .map_err(AppError::ZBusError)?;

        let result = devs_path
            .into_iter()
            .map(|d| NmDevice::from((d, self)))
            .collect::<Vec<_>>();

        Ok(result)
    }

    // pub async fn wifi<'a>(&'a self, path: &'a str) -> Result<NmWifi<'a>, AppError> {
    //     let proxy = self.proxy(path, NmIface::DEVICE_WIRELESS).await?;
    //     Ok(NmWifi {
    //         proxy,
    //         device: self,
    //     })
    // }
}

impl<'a> NmDevice<'a> {
    pub async fn interface(&self) -> Option<String> {
        self.get_property::<String>(self.device, "Interface")
            .await
            .ok()
    }

    pub async fn device_type(&self) -> Result<NmDeviceType, AppError> {
        let device_type = self.get_property::<u32>(self.device, "DeviceType").await?;

        Ok(NmDeviceType { device_type })
    }

    pub async fn state(&self) -> Option<u32> {
        self.get_property::<u32>(self.device, "State").await.ok()
    }

    pub async fn wireless(&'a self) -> Option<NmWifi<'a>> {
        let proxy = self
            .device
            .proxy(self.path(), NmIface::DEVICE_WIRELESS)
            .await
            .ok()?;

        let object_path = proxy
            .get_property::<OwnedObjectPath>("ActiveAccessPoint")
            .await
            .map_err(AppError::ZBusError)
            .ok()?;

        if object_path.as_str() == "/" {
            return None;
        }

        Some(NmWifi {
            object_path,
            device: self.device,
        })
    }

    pub async fn active(&'a self) -> Result<NmActive<'a>, AppError> {
        let path = self
            .get_property::<OwnedObjectPath>(self.device, "ActiveConnection")
            .await?;

        Ok(NmActive {
            object_path: path,
            device: self.device,
        })
    }
}

#[derive(Copy, Clone)]
pub struct NmDeviceType {
    device_type: u32,
}

impl From<NmDeviceType> for DevType {
    fn from(nm: NmDeviceType) -> DevType {
        match nm.device_type {
            1 => DevType::Ethernet,
            2 => DevType::WiFi,
            5 => DevType::Bluetooth,
            _ => DevType::Other,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum DevType {
    Ethernet,
    WiFi,
    Bluetooth,
    Other,
}

impl DevType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DevType::Ethernet => "Ethernet",
            DevType::WiFi => "WiFi",
            DevType::Bluetooth => "Bluetooth",
            DevType::Other => "Other",
        }
    }
}

impl NmDeviceType {
    fn to_dev_type(self) -> DevType {
        DevType::from(self)
    }
    pub fn name(&self) -> &'static str {
        self.to_dev_type().as_str()
    }

    pub fn is_wifi(&self) -> bool {
        self.to_dev_type() == DevType::WiFi
    }

    pub fn is_ethernet(&self) -> bool {
        self.to_dev_type() == DevType::Ethernet
    }
}

pub struct NmActive<'a> {
    object_path: OwnedObjectPath,
    device: &'a Nm,
}

impl_nm_child!(NmActive, NmIface::ACTIVE_CONNECTION);

impl<'a> NmActive<'a> {
    pub async fn ipv4_conf(&self) -> Option<NmAddress<'a>> {
        if self.object_path.as_str() != "/" {
            self.get_property::<OwnedObjectPath>(self.device, "Ip4Config")
                .await
                .ok()
                .map(|object_path| NmAddress {
                    device: self.device,
                    object_path,
                })
        } else {
            None
        }
    }
}

pub struct NmAddress<'a> {
    object_path: OwnedObjectPath,
    device: &'a Nm,
}
impl_nm_child!(NmAddress, NmIface::IP4_CONFIG);

impl<'a> NmAddress<'a> {
    async fn address(&self) -> Result<Vec<HashMap<String, Value<'a>>>, AppError> {
        self.get_property::<Vec<HashMap<String, Value>>>(self.device, "AddressData")
            .await
    }

    pub async fn ipv4(&self) -> Option<String> {
        let address = self.address().await.ok()?;
        address.first()?.get("address").and_then(|v| match v {
            Value::Str(addr) => Some(addr.to_string()),
            _ => None,
        })
    }
}

impl_nm_child!(NmWifi, NmIface::ACCESSPOINT);
pub struct NmWifi<'a> {
    object_path: OwnedObjectPath,
    device: &'a Nm,
}

impl<'a> NmWifi<'a> {
    pub async fn ssid(&self) -> Result<String, AppError> {
        let ssid = self.get_property::<Vec<u8>>(self.device, "Ssid").await?;
        Ok(String::from_utf8_lossy(&ssid).to_string())
    }

    pub async fn strength(&self) -> Result<u8, AppError> {
        self.get_property::<u8>(self.device, "Strength").await
    }
}

//==============

pub struct NmWifiActiveAccessPoint {
    object_path: OwnedObjectPath,
}

impl NmWifiActiveAccessPoint {
    pub fn path(&self) -> &str {
        self.object_path.as_str()
    }
}

pub struct NmAccessPoint<'a> {
    proxy: Proxy<'a>,
}

impl<'a> NmAccessPoint<'a> {
    pub async fn ssid(&self) -> Result<Vec<u8>, AppError> {
        self.proxy
            .get_property::<Vec<u8>>("Ssid")
            .await
            .map_err(AppError::ZBusError)
    }

    pub async fn strength(&self) -> Result<u8, AppError> {
        self.proxy
            .get_property::<u8>("Strength")
            .await
            .map_err(AppError::ZBusError)
    }
}
