use crate::{
    db::connection::Connection,
    db::primary::{repository::PrimaryDbRepository, service::Service},
    sensors::{
        model::{Sensor, Status},
        repository::SensorRepository,
        service::SensorService,
    },
};
use anyhow::{Ok, Result};

/// Maps a typed sensor error into an anyhow error for test ergonomics.
fn anyhow_err(e: crate::sensors::error::SensorError) -> anyhow::Error {
    anyhow::anyhow!("{}", e)
}

/// Only rows matching the requested status are returned, whatever their
/// insertion path (service creation or direct repository seeding).
#[tokio::test]
async fn should_filter_by_status() -> Result<()> {
    let connection = Connection::new().await?;
    let repo = SensorRepository::new(connection.primary_db_client().clone());
    let service = SensorService::new(connection.clone());

    // The service only ever creates InProgress rows; other statuses are
    // seeded through the repository to cover every filter branch.
    service.add("00:1B:44:11:3A:C1").await.map_err(anyhow_err)?;
    repo.insert(&Sensor::new(
        0,
        "00:1B:44:11:3A:C2".to_string(),
        Status::Installed,
    ))
    .await?;
    repo.insert(&Sensor::new(
        0,
        "00:1B:44:11:3A:C3".to_string(),
        Status::InProgress,
    ))
    .await?;
    repo.insert(&Sensor::new(
        0,
        "00:1B:44:11:3A:C4".to_string(),
        Status::OnUpdate,
    ))
    .await?;

    let mut in_progress = service
        .list_by_status(&Status::InProgress)
        .await
        .map_err(anyhow_err)?;
    in_progress.sort_by(|a, b| a.mac.cmp(&b.mac));
    assert_eq!(in_progress.len(), 2);
    assert_eq!(in_progress[0].mac, "00:1B:44:11:3A:C1");
    assert_eq!(in_progress[1].mac, "00:1B:44:11:3A:C3");

    let installed = service
        .list_by_status(&Status::Installed)
        .await
        .map_err(anyhow_err)?;
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].mac, "00:1B:44:11:3A:C2");

    Ok(())
}

/// A fresh database yields an empty vector rather than an error.
#[tokio::test]
async fn should_return_empty_when_no_match() -> Result<()> {
    let connection = Connection::new().await?;
    let service = SensorService::new(connection);

    let sensors = service
        .list_by_status(&Status::InProgress)
        .await
        .map_err(anyhow_err)?;
    assert!(sensors.is_empty());
    Ok(())
}
