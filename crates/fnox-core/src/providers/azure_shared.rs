use azure_core::credentials::{Secret, TokenCredential};
use azure_identity::{
    ClientSecretCredential, DeveloperToolsCredential, WorkloadIdentityCredential,
};
use std::sync::Arc;

pub const AUTH_HINT: &str = "Run 'az login', or set AZURE_TENANT_ID and AZURE_CLIENT_ID with AZURE_CLIENT_SECRET or AZURE_FEDERATED_TOKEN_FILE";

#[derive(Debug, PartialEq)]
enum Source {
    ClientSecret {
        tenant_id: String,
        client_id: String,
        client_secret: String,
    },
    WorkloadIdentity,
    DeveloperTools,
}

/// Chosen from the environment rather than by trying each credential in turn, so a
/// machine without Azure metadata never waits on a managed identity probe. The order
/// follows Azure's `EnvironmentCredential` and `DefaultAzureCredential`.
fn source(var: impl Fn(&str) -> Option<String>) -> Source {
    let var = |name: &str| var(name).filter(|value| !value.is_empty());
    if let (Some(tenant_id), Some(client_id), Some(client_secret)) = (
        var("AZURE_TENANT_ID"),
        var("AZURE_CLIENT_ID"),
        var("AZURE_CLIENT_SECRET"),
    ) {
        return Source::ClientSecret {
            tenant_id,
            client_id,
            client_secret,
        };
    }
    if var("AZURE_FEDERATED_TOKEN_FILE").is_some() {
        return Source::WorkloadIdentity;
    }
    Source::DeveloperTools
}

pub fn has_environment_credentials() -> bool {
    source(|name| crate::env::var(name).ok()) != Source::DeveloperTools
}

/// The credential every Azure provider and the `azure-token` lease authenticate with.
pub fn credential() -> azure_core::Result<Arc<dyn TokenCredential>> {
    Ok(match source(|name| crate::env::var(name).ok()) {
        Source::ClientSecret {
            tenant_id,
            client_id,
            client_secret,
        } => ClientSecretCredential::new(&tenant_id, client_id, Secret::new(client_secret), None)?,
        // Reads AZURE_TENANT_ID, AZURE_CLIENT_ID and the token file itself, re-reading
        // the file when the token it holds expires.
        Source::WorkloadIdentity => WorkloadIdentityCredential::new(None)?,
        Source::DeveloperTools => DeveloperToolsCredential::new(None)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    const SERVICE_PRINCIPAL: [(&str, &str); 3] = [
        ("AZURE_TENANT_ID", "tenant"),
        ("AZURE_CLIENT_ID", "client"),
        ("AZURE_CLIENT_SECRET", "secret"),
    ];

    #[test]
    fn falls_back_to_developer_tools_without_environment_credentials() {
        assert_eq!(source(lookup(&[])), Source::DeveloperTools);
    }

    #[test]
    fn uses_a_client_secret_when_all_three_variables_are_set() {
        assert_eq!(
            source(lookup(&SERVICE_PRINCIPAL)),
            Source::ClientSecret {
                tenant_id: "tenant".into(),
                client_id: "client".into(),
                client_secret: "secret".into(),
            }
        );
    }

    #[test]
    fn ignores_a_partial_client_secret_configuration() {
        let vars = [
            ("AZURE_CLIENT_ID", "client"),
            ("AZURE_CLIENT_SECRET", "secret"),
        ];
        assert_eq!(source(lookup(&vars)), Source::DeveloperTools);
    }

    #[test]
    fn uses_workload_identity_when_a_federated_token_file_is_set() {
        let vars = [
            ("AZURE_TENANT_ID", "tenant"),
            ("AZURE_CLIENT_ID", "client"),
            ("AZURE_FEDERATED_TOKEN_FILE", "/run/token"),
        ];
        assert_eq!(source(lookup(&vars)), Source::WorkloadIdentity);
    }

    #[test]
    fn prefers_a_client_secret_over_a_federated_token_file() {
        let mut vars = SERVICE_PRINCIPAL.to_vec();
        vars.push(("AZURE_FEDERATED_TOKEN_FILE", "/run/token"));
        assert!(matches!(source(lookup(&vars)), Source::ClientSecret { .. }));
    }

    #[test]
    fn treats_empty_variables_as_unset() {
        let vars = [
            ("AZURE_TENANT_ID", ""),
            ("AZURE_CLIENT_ID", ""),
            ("AZURE_CLIENT_SECRET", ""),
            ("AZURE_FEDERATED_TOKEN_FILE", ""),
        ];
        assert_eq!(source(lookup(&vars)), Source::DeveloperTools);
    }
}
