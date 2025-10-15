use bluer::{
    adv::Advertisement,
    gatt::local::{Application, Characteristic, CharacteristicRead, Service},
};
use futures::FutureExt;
use std::time::Duration;
use tokio::time::sleep;

use ble_net_mngmt::handle_read_request;

const SERVICE_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x12345678_1234_5678_1234_56789abcdef0);
const CHARACTERISTIC_UUID: uuid::Uuid =
    uuid::Uuid::from_u128(0x12345678_1234_5678_1234_56789abcdef1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Запуск Bluetooth LE сервера...");
    let _ = ble_net_mngmt::network::collect::collect_network_interface().await;

    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;
    adapter.set_powered(true).await?;

    println!("Адаптер: {}", adapter.name());
    println!("Адрес: {}", adapter.address().await?);

    // Создаем GATT сервис
    let app = Application {
        services: vec![Service {
            uuid: SERVICE_UUID,
            primary: true,
            characteristics: vec![Characteristic {
                uuid: CHARACTERISTIC_UUID,
                read: Some(CharacteristicRead {
                    read: true,
                    fun: Box::new(|req| handle_read_request(req).boxed()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Регистрируем GATT приложение
    let app_handle = adapter.serve_gatt_application(app).await?;

    println!("GATT сервис зарегистрирован");
    println!("Service UUID: {}", SERVICE_UUID);
    println!("Characteristic UUID: {}", CHARACTERISTIC_UUID);

    // Настраиваем рекламу (advertisement)
    let le_advertisement = Advertisement {
        service_uuids: vec![SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("HelloWorld BLE".to_string()),
        ..Default::default()
    };

    // Запускаем рекламу
    let adv_handle = adapter.advertise(le_advertisement).await?;

    println!("\n✅ BLE сервер запущен!");
    println!("Устройство доступно как: 'HelloWorld BLE'");
    println!("Подключитесь к устройству и прочитайте характеристику для получения 'Hello World'");
    println!("\nНажмите Ctrl+C для остановки...\n");

    // Держим сервер активным
    loop {
        sleep(Duration::from_secs(60)).await;
    }

    // Cleanup
    drop(adv_handle);
    drop(app_handle);

    Ok(())
}
