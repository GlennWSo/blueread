use std::error::Error;
use std::time::Duration;

use bluest::{btuuid, Adapter, Uuid};
use futures_lite::StreamExt;
use tracing::info;
use tracing::metadata::LevelFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{fmt, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let adapter = Adapter::default()
        .await
        .ok_or("Bluetooth adapter not found")?;
    adapter.wait_available().await?;

    info!("starting scan");
    let service_filter = [btuuid::bluetooth_uuid_from_u16(0x185A)];
    let mut scan = adapter.scan(&service_filter).await?;
    info!("scan started");
    while let Some(device) = scan.next().await {
        info!(
            "{:?} {}{}: {:?}",
            device.device.id(),
            device.device.name().as_deref().unwrap_or("(unknown)"),
            device
                .rssi
                .map(|x| format!(" ({}dBm)", x))
                .unwrap_or_default(),
            device.adv_data.services
        );
        adapter.connect_device(&device.device).await?;
        info!("connected!");

        let services = device.device.services().await?;

        for service in services {
            let id = service.uuid();
            info!("{:?} {:#?}", id, service);
            if id == Uuid::parse_str("0000185a-0000-1000-8000-00805f9b34fb")? {
                println!("bingo");
                let characteristics = service.characteristics().await?;
                for characteristic in characteristics {
                    info!("    {:?}", characteristic);

                    let props = characteristic.properties().await?;
                    info!("      props: {:?}", props);
                    if props.read {
                        info!("      value: {:?}", characteristic.read().await);
                    }

                    info!("enabling button notifications");

                    let mut updates = characteristic.notify().await?;

                    info!("waiting for button changes");

                    let mut i: u8 = 0;
                    while let Some(val) = updates.next().await {
                        let val = val?;
                        let bytes: [u8; 4] = val.try_into().unwrap();
                        let val: f32 = f32::from_le_bytes(bytes);
                        info!("rpm state: {:?}", val);
                        i += 1;
                        if i > 30 {
                            break;
                        }
                    }
                }
                break;
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;

        adapter.disconnect_device(&device.device).await?;
        info!("disconnected!");
    }

    Ok(())
}
