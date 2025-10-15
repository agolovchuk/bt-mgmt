use bluer::{
    Adapter, Error, Session, Uuid,
    adv::Advertisement,
    gatt::local::{Application, Characteristic, CharacteristicRead, Service},
};
use futures::FutureExt;
use std::collections::BTreeMap;

// async fn collect_netifs_json() -> String {}

#[derive(serde::Serialize)]
struct Msg {
    pub message: String,
}

#[derive(Default)]
pub struct Ble {
    service_uuid: Uuid,
    char_uuid: Uuid,
    adapter: Option<Adapter>,
}

impl Ble {
    pub fn to_uuid(name: &str) -> Uuid {
        let no_var_msg = format!("No variable {} in env", name);
        let incorrect_var_msg = format!("Incorrect {}", name);
        Uuid::parse_str(&std::env::var(name).expect(&no_var_msg)).expect(&incorrect_var_msg)
    }

    pub fn new() -> Self {
        // let service_uuid = Ble::to_uuid("SERVICE_UUID");
        // let char_uuid = Ble::to_uuid("CHAR_UUID");
        Self {
            service_uuid: Uuid::from_u128(0xFEEDC0DE),
            char_uuid: Uuid::from_u128(0xF00DC0DE00001),
            adapter: None,
        }
    }

    pub async fn build(&mut self) -> Result<&Self, Error> {
        let session = Session::new().await?;
        let adapter: Adapter = session.default_adapter().await?;
        adapter.set_powered(true).await?;
        self.adapter = Some(adapter);
        Ok(self)
    }

    pub async fn adv(&self) -> Result<bluer::adv::AdvertisementHandle, Error> {
        let mut manufacturer_data = BTreeMap::new();
        manufacturer_data.insert(0xf00d, vec![0x21, 0x22, 0x23, 0x24]);

        let le_advertisement = Advertisement {
            service_uuids: vec![self.service_uuid].into_iter().collect(),
            manufacturer_data,
            discoverable: Some(true),
            local_name: Some(std::env::var("DEVICE_NAME").expect("No DEVICE_NAME in env")),
            ..Default::default()
        };

        if let Some(adapter) = &self.adapter {
            let adv_handle: bluer::adv::AdvertisementHandle =
                adapter.advertise(le_advertisement).await?;

            let app = Application {
                services: vec![Service {
                    uuid: self.service_uuid,
                    primary: true,
                    characteristics: vec![Characteristic {
                        uuid: self.char_uuid,
                        read: Some(CharacteristicRead {
                            read: true,
                            encrypt_read: false,
                            // При каждом чтении собираем актуальное состояние интерфейсов
                            fun: Box::new(|_req| {
                                async move {
                                    // let msg = Msg {
                                    //     message: "Hello world".to_string(),
                                    // };
                                    // let json = collect_netifs_json().await;
                                    // let json = serde_json::to_string(&msg)
                                    //     .unwrap_or_else(|_| "[]".to_string());
                                    // Ok(json.into_bytes())
                                    println!("Получен запрос на чтение от устройства");
                                    Ok(b"Hello World".to_vec())
                                    // Ok("Hello world".to_string().into_bytes())
                                }
                                .boxed()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            };

            let _app_handle = adapter.serve_gatt_application(app).await?;
            return Ok(adv_handle);
        }
        Err(Error {
            kind: bluer::ErrorKind::Failed,
            message: "Sh H".to_string(),
        })
    }
}
