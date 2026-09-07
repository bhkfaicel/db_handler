use crate::db::primary::{models::SortOrder, service::Service};
use crate::sensors::model::{PaginationParams, Sensor, SensorSortKey, Status};
use crate::{db::connection::Connection, sensors::service::SensorService};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_return_sensors() -> Result<()> {
    let connection = Connection::new().await?;
    let mut sensors: Vec<Sensor> = Vec::new();
    let service = SensorService::new(connection.clone());
    for i in 1..30 {
        // Well-formed six-hex-pair MAC: the MAC format is a validated
        // invariant of the registration boundary, so fixtures must carry
        // realistic addresses (2-digit uppercase hex suffix, unique per i).
        let mac = format!("00:00:00:00:00:{:02X}", i);
        sensors.push(Sensor {
            id: i,
            mac: mac.clone(),
            status: Status::InProgress,
            sensor_type: None,
        });
        service
            .add(&mac)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: -1,
        sort_key: SensorSortKey::Id,
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let results = service
        .find_by_string(&pagination_params)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    assert_eq!(results.total, 29);
    assert_eq!(results.items, sensors);
    Ok(())
}
