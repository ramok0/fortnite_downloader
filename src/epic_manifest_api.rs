use serde::Deserialize;
use serde::Serialize;
pub const CATALOG_ITEM: &str = "4fe75bbc5a674f4f9b356b5c90567da5";
pub const EPIC_MANIFEST_API: &'static str = "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/v2/platform/Windows/namespace/fn/catalogItem/4fe75bbc5a674f4f9b356b5c90567da5/app/Fortnite/label/Live";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpicManifestResponse  {
    pub elements: Vec<ManifestList>,
}

impl EpicManifestResponse {
    pub fn get_first_manifest(&self) -> Option<String> {
        self.elements.first().and_then(|x| x.manifests.first()).and_then(|x| Some(x.get_uri()))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestList {
    pub app_name: String,
    pub label_name: String,
    pub build_version: String,
    pub hash: String,
    pub use_signed_url: bool,
    pub metadata: Metadata,
    pub manifests: Vec<Manifest>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub installation_pool_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub uri: String,
    pub query_params: Vec<QueryParam>,
}

impl Manifest {
    pub fn get_uri(&self) -> String {
        let mut uri = self.uri.clone();
        if self.query_params.len() > 0 {
            uri += "?";
        }
        for query_param in &self.query_params {
            uri += &format!("&{}={}", query_param.name, query_param.value); 
        }
        uri
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryParam {
    pub name: String,
    pub value: String,
}