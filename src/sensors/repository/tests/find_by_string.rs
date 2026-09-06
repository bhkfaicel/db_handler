use crate::{
    db::connection::Connection,
    db::primary::{models::SortOrder, repository::PrimaryDbRepository},
    sensors::{
        model::{PaginationParams, Sensor, SensorSortKey, Status},
        repository::SensorRepository,
    },
};
use anyhow::{Ok, Result};

#[tokio::test]
async fn should_return_sensors() -> Result<()> {
    let connection = Connection::new().await?;
    let pool = connection.primary_db_client().clone();
    let repository = SensorRepository::new(pool);
    let mut sensors: Vec<Sensor> = Vec::new();
    for i in 1..30 {
        let sensor = Sensor::new(0, i.to_string(), Status::Installed);
        sensors.push(Sensor {
            id: i,
            mac: i.to_string(),
            status: Status::Installed,
            sensor_type: None,
        });
        repository.insert(&sensor).await?;
    }
    let pagination_params = PaginationParams {
        skip: 0,
        nbr: -1,
        sort_key: SensorSortKey::Id,
        sort: SortOrder::Asc,
        searched_value: None,
    };
    let result = repository.find_by_string(&pagination_params).await?;
    assert_eq!(result.total, 29);
    assert_eq!(result.items, sensors);
    Ok(())
}
