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
    pub const WIRELESS_ACCESSPOINT: &'static str = "org.freedesktop.NetworkManager.AccessPoint";
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
            .map_err(AppError::NmError)
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
            .map_err(AppError::NmError)?;

        let result = devs_path
            .into_iter()
            .map(|d| NmDevice::from((d, self)))
            .collect::<Vec<_>>();

        Ok(result)
    }

    pub async fn wifi<'a>(&'a self, path: &'a str) -> Result<NmWifi<'a>, AppError> {
        let proxy = self.proxy(path, NmIface::DEVICE_WIRELESS).await?;
        Ok(NmWifi {
            proxy,
            device: self,
        })
    }
}

impl<'a> NmDevice<'a> {
    pub async fn interface(&self) -> Option<String> {
        self.to_proxy(self.device)
            .await
            .ok()?
            .get_property::<String>("Interface")
            .await
            .ok()
    }

    pub async fn device_type(&self) -> Result<NmDeviceType, AppError> {
        let device_type = self
            .to_proxy(self.device)
            .await?
            .get_property::<u32>("DeviceType")
            .await
            .map_err(AppError::NmError)?;

        Ok(NmDeviceType { device_type })
    }

    pub async fn state(&self) -> Option<u32> {
        self.to_proxy(self.device)
            .await
            .ok()?
            .get_property::<u32>("State")
            .await
            .ok()
    }

    pub async fn active(&'a self) -> Result<NmActive<'a>, AppError> {
        let path = self
            .to_proxy(self.device)
            .await?
            .get_property::<OwnedObjectPath>("ActiveConnection")
            .await
            .map_err(AppError::NmError)?;

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
            self.to_proxy(self.device)
                .await
                .ok()?
                .get_property::<OwnedObjectPath>("Ip4Config")
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
        self.to_proxy(self.device)
            .await?
            .get_property::<Vec<HashMap<String, Value>>>("AddressData")
            .await
            .map_err(AppError::NmError)
    }

    pub async fn ipv4(&self) -> Option<String> {
        let address = self.address().await.ok()?;
        address.first()?.get("address").and_then(|v| match v {
            Value::Str(addr) => Some(addr.to_string()),
            _ => None,
        })
    }
}

//==============

pub struct NmWifi<'a> {
    proxy: Proxy<'a>,
    device: &'a Nm,
}

pub struct NmWifiActiveAccessPoint {
    object_path: OwnedObjectPath,
}

impl NmWifiActiveAccessPoint {
    pub fn path(&self) -> &str {
        self.object_path.as_str()
    }
}

impl<'a> NmWifi<'a> {
    pub async fn active_access_point(&self) -> Result<NmWifiActiveAccessPoint, AppError> {
        let object_path = self
            .proxy
            .get_property::<OwnedObjectPath>("ActiveAccessPoint")
            .await
            .map_err(AppError::NmError)?;
        Ok(NmWifiActiveAccessPoint { object_path })
    }

    pub async fn ap(
        &self,
        wifi_aap: &'a NmWifiActiveAccessPoint,
    ) -> Result<NmAccessPoint<'a>, AppError> {
        let proxy = self
            .device
            .proxy(wifi_aap.path(), NmIface::WIRELESS_ACCESSPOINT)
            .await?;
        Ok(NmAccessPoint { proxy })
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
            .map_err(AppError::NmError)
    }

    pub async fn strength(&self) -> Result<u8, AppError> {
        self.proxy
            .get_property::<u8>("Strength")
            .await
            .map_err(AppError::NmError)
    }
}
