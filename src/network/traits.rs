use async_trait::async_trait;
use zbus::{Error, Proxy};
use zvariant::OwnedValue;

use super::nm_device::Nm;
use crate::error::AppError;

#[async_trait]
pub trait NmChildObject<'a>: Sized {
    const IFACE: &'static str;

    fn path(&self) -> &str;

    async fn to_proxy(&'a self, device: &'a Nm) -> Result<Proxy<'a>, AppError> {
        device.proxy(self.path(), Self::IFACE).await
    }

    async fn get_property<T>(&'a self, device: &'a Nm, name: &str) -> Result<T, AppError>
    where
        T: TryFrom<OwnedValue>,
        T::Error: Into<Error>,
    {
        let proxy = self.to_proxy(device).await?;

        proxy
            .get_property::<T>(name)
            .await
            .map_err(AppError::ZBusError)
    }
}

#[macro_export]
macro_rules! impl_nm_child {
    ($model_mod:ident, $iface:expr) => {
        #[async_trait::async_trait]
        impl<'a> NmChildObject<'a> for $model_mod<'a> {
            // const PROPERTY: &'static str = $property;
            const IFACE: &'static str = $iface;

            fn path(&self) -> &str {
                self.object_path.as_str()
            }

            // async fn from_parent(parent: &Proxy<'a>) -> Result<Self, AppError> {
            //     let object_path = parent
            //         .get_property::<OwnedObjectPath>(Self::PROPERTY)
            //         .await
            //         .map_err(AppError::NmError)?;
            //     Ok(Self { object_path })
            // }

            async fn to_proxy(&'a self, device: &'a Nm) -> Result<Proxy<'a>, AppError> {
                device.proxy(self.path(), Self::IFACE).await
            }
        }
    };
}
