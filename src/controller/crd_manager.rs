use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    Client, CustomResourceExt,
    api::{Api, PostParams},
};
use tracing::{info, warn};

pub async fn init_crds<T>(client: Client) -> anyhow::Result<()>
where
    T: CustomResourceExt,
{
    let crd = T::crd();
    let name = crd.metadata.name.as_ref().unwrap();
    let api: Api<CustomResourceDefinition> = Api::all(client);

    info!("Checking CRD: {}", name);

    if let Ok(existing) = api.get(name).await {
        info!("CRD {} already exists, merging versions", name);
        let mut new_crd = crd.clone();

        // Simple merge strategy: keep existing versions that are not in the new CRD
        // This is a basic implementation. A more robust one would check for version compatibility.
        let existing_spec = existing.spec;
        let mut versions = new_crd.spec.versions;
        for existing_version in existing_spec.versions {
            if !versions.iter().any(|v| v.name == existing_version.name) {
                warn!("Preserving old version: {}", existing_version.name);
                versions.push(existing_version);
            }
        }
        new_crd.spec.versions = versions;

        api.replace(name, &PostParams::default(), &new_crd).await?;
        info!("CRD {} updated", name);
    } else {
        info!("Creating CRD: {}", name);
        api.create(&PostParams::default(), &crd).await?;
        info!("CRD {} created", name);
    }

    Ok(())
}
