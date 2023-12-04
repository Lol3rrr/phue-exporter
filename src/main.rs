use std::{sync::Arc, time::Duration};

use axum::extract::State;
use clap::Parser;
use phue_exporter::Bridge;
use tracing_subscriber::{layer::SubscriberExt, Layer};

#[derive(Debug, Parser)]
enum PHueCli {
    Register,
    Run,
}

struct Metrics {
    registry: prometheus::Registry,
}

fn main() -> Result<(), ()> {
    let cli = PHueCli::parse();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
                tracing::Level::INFO,
            )),
    );
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let addr = match std::env::var("HUE_ADDR") {
        Ok(a) => a,
        Err(_) => {
            tracing::error!("Missing 'HUE_ADDR' environment variable");
            return Err(());
        }
    };

    let http_client = reqwest::Client::builder().build().unwrap();

    match cli {
        PHueCli::Register => {
            tracing::info!("Registering with hue bridge");

            match runtime.block_on(Bridge::register(&http_client, addr)) {
                Ok(u) => {
                    tracing::info!("Registered with username: {:?}", u);
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Error: {:?}", e);
                    Err(())
                }
            }
        }
        PHueCli::Run => {
            tracing::info!("Running Exporter");

            let username = match std::env::var("HUE_USER") {
                Ok(u) => u,
                Err(_) => {
                    tracing::error!("Missing 'HUE_USER' environment variable");
                    return Err(());
                }
            };

            let bridge = Bridge::new(http_client, addr, username);

            let shared_metrics = Arc::new(Metrics {
                registry: prometheus::Registry::new_custom(Some("phue".to_string()), None).unwrap(),
            });

            let app = axum::Router::new()
                .route("/metrics", axum::routing::get(metrics))
                .with_state(Arc::clone(&shared_metrics));

            runtime.spawn(updating(bridge, shared_metrics));

            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("0.0.0.0:9292").await.unwrap();
                axum::serve(listener, app).await.unwrap();
            });

            Ok(())
        }
    }
}

#[tracing::instrument(skip(bridge, metrics))]
async fn updating(bridge: Bridge, metrics: Arc<Metrics>) {
    let label_names = &["unique_id", "name"];
    let lights_on = prometheus::GaugeVec::new(
        prometheus::Opts::new("lights_on", "The State of Lights"),
        label_names,
    )
    .unwrap();

    let lights_brightness = prometheus::GaugeVec::new(
        prometheus::Opts::new("lights_brightness", "The Brightness of Lights"),
        label_names,
    )
    .unwrap();

    metrics
        .registry
        .register(Box::new(lights_on.clone()))
        .unwrap();
    metrics
        .registry
        .register(Box::new(lights_brightness.clone()))
        .unwrap();

    loop {
        tracing::info!("Updating Metrics");

        match bridge.lights().await {
            Ok(lights) => {
                for (_, light) in lights {
                    lights_on
                        .with_label_values(&[&light.uniqueid, &light.name])
                        .set(if light.state.on { 1.0 } else { 0.0 });

                    lights_brightness
                        .with_label_values(&[&light.uniqueid, &light.name])
                        .set(light.state.bri as f64);
                }
            }
            Err(e) => {
                tracing::error!("Loading Lights: {:?}", e);
            }
        };

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

#[tracing::instrument(skip(state))]
async fn metrics(State(state): State<Arc<Metrics>>) -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = state.registry.gather();
    match encoder.encode_to_string(&metric_families) {
        Ok(r) => r,
        Err(e) => {
            println!("Error encoding: {:?}", e);
            String::new()
        }
    }
}
