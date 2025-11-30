use kube::{
    runtime::{controller::Action, Controller},
    Api, Client, ResourceExt,
};
use std::sync::Arc;
use tracing::{info, error};
use futures::StreamExt;
use crate::shared::context::Context;
use crate::shared::resources::common::{self, MonitorResource, ControllerResource};
use crate::shared::settings::Settings;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to reconcile: {0}")]
    Anyhow(#[from] anyhow::Error),
}

pub async fn reconcile<T>(obj: Arc<T>, ctx: Arc<Context>) -> Result<Action, Error>
where
    T: MonitorResource + serde::Serialize + std::fmt::Debug,
{
    // Validate the resource
    obj.validate().map_err(Error::Anyhow)?;

    let client = reqwest::Client::new();
    let worker_url = common::build_worker_url::<T>(&ctx.settings.controller.base_url);

    info!("Dispatching {} {} to worker at {}", T::kind(&()), obj.name_any(), worker_url);

    let res = client.post(&worker_url)
        .json(&*obj)
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                info!("Successfully dispatched to worker");
            } else {
                error!("Worker returned error: {:?}", response.status());
            }
        }
        Err(e) => {
            error!("Failed to dispatch to worker: {:?}", e);
        }
    }

    Ok(obj.success_policy())
}

pub fn error_policy<T>(obj: Arc<T>, error: &Error, ctx: Arc<Context>) -> Action
where
    T: ControllerResource,
{
    match error {
        Error::Anyhow(e) => obj.error_policy(e, ctx),
    }
}

pub fn run_monitor_controller<T>(client: Client, settings: Settings) -> impl futures::Future<Output = ()>
where
    T: MonitorResource + serde::Serialize + std::fmt::Debug + serde::de::DeserializeOwned,
{
    let monitors = Api::<T>::all(client.clone());
    let context = Arc::new(Context {
        client: client.clone(),
        settings: settings.clone(),
    });

    Controller::new(monitors, Default::default())
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => info!("reconcile failed: {:?}", e),
            }
        })
}

pub async fn reconcile_notifier<T>(obj: Arc<T>, _ctx: Arc<Context>) -> Result<Action, Error>
where
    T: ControllerResource + serde::Serialize + std::fmt::Debug,
{
    // Validate the resource
    obj.validate().map_err(Error::Anyhow)?;
    
    // Notifiers are passive, so we just return the success policy
    Ok(obj.success_policy())
}



pub fn run_notifier_controller<T>(client: Client, settings: Settings) -> impl futures::Future<Output = ()>
where
    T: ControllerResource + serde::Serialize + std::fmt::Debug + serde::de::DeserializeOwned,
{
    let notifiers = Api::<T>::all(client.clone());
    let context = Arc::new(Context {
        client: client.clone(),
        settings: settings.clone(),
    });

    Controller::new(notifiers, Default::default())
        .run(reconcile_notifier, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled notifier {:?}", o),
                Err(e) => info!("notifier reconcile failed: {:?}", e),
            }
        })
}
