pub mod ble;
pub mod error;
pub mod network;

use bluer::gatt::local::ReqError;

use crate::network::collect::collect_network_interface;

pub async fn handle_read_request(
    req: bluer::gatt::local::CharacteristicReadRequest,
) -> Result<Vec<u8>, ReqError> {
    println!("📖 Получен запрос на чтение");

    // // Можно использовать req для получения дополнительной информации
    println!("   Request: {:?}", req);

    // println!("   Возвращаем: 'Hello World'");
    // Ok(b"Hello World".to_vec())
    collect_network_interface().await.map_err(ReqError::from)
}
