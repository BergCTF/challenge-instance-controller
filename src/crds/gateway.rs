use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// HTTPRoute CRD (Gateway API v1)
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "gateway.networking.k8s.io",
    version = "v1",
    kind = "HTTPRoute",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct HTTPRouteSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_refs: Option<Vec<ParentReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<HTTPRouteRule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HTTPRouteRule {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_refs: Option<Vec<HTTPBackendRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Vec<HTTPRouteFilter>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HTTPRouteFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_redirect: Option<HTTPRequestRedirectFilter>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HTTPRequestRedirectFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
}

/// TLSRoute CRD (Gateway API v1alpha2)
#[derive(CustomResource, Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "gateway.networking.k8s.io",
    version = "v1alpha2",
    kind = "TLSRoute",
    namespaced
)]
#[serde(rename_all = "camelCase")]
pub struct TLSRouteSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostnames: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_refs: Option<Vec<ParentReference>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<TLSRouteRule>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TLSRouteRule {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_refs: Option<Vec<BackendRef>>,
}

/// Common types

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParentReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BackendRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HTTPBackendRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<i32>,
}
