---
description: "Read Azure Key Vault secrets with fnox. Configure vault access, authentication, prefixes, and profiles."
---

# Azure Key Vault secrets

Azure Key Vault Secrets provides centralized secret management for Azure workloads.

## Quick start

```sh
# Create Key Vault
az keyvault create --name "myapp-vault" --resource-group "myapp-rg"
```

Add these definitions to `fnox.toml`. Merge them into any existing tables with the same names:

```toml
[providers]
azure = { type = "azure-sm", vault_url = "https://myapp-vault.vault.azure.net/", prefix = "myapp-" }
```

```sh
# Create secret
az keyvault secret set --vault-name "myapp-vault" --name "myapp-database-url" --value "postgresql://..."
```

Add these definitions to `fnox.toml`. Merge them into any existing tables with the same names:

```toml
[secrets]
DATABASE_URL = { provider = "azure", value = "database-url" }
```

```sh
# Get secret
fnox get DATABASE_URL
```

## Authentication

fnox picks the credential from the environment, in this order:

1. Service principal: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID` and `AZURE_CLIENT_SECRET` (all three)
2. Workload identity: `AZURE_TENANT_ID`, `AZURE_CLIENT_ID` and `AZURE_FEDERATED_TOKEN_FILE`
3. Azure CLI (`az login`), then Azure Developer CLI (`azd auth login`)

```bash
# Development
az login

# Service principal (CI/CD)
export AZURE_TENANT_ID="..."
export AZURE_CLIENT_ID="..."
export AZURE_CLIENT_SECRET="..."
```

Workload identity needs no secret and no `az`. Any OIDC token the app registration trusts
through a federated credential works, such as one from GitLab CI:

```yaml
read-secrets:
  id_tokens:
    AZURE_ID_TOKEN:
      aud: api://AzureADTokenExchange
  variables:
    AZURE_TENANT_ID: "..."
    AZURE_CLIENT_ID: "..."
    AZURE_FEDERATED_TOKEN_FILE: /tmp/azure-token
  script:
    - printf '%s' "$AZURE_ID_TOKEN" > "$AZURE_FEDERATED_TOKEN_FILE"
    - fnox exec -- ./deploy.sh
```

On AKS with workload identity enabled, the webhook sets all three variables for you.

Managed identity is not supported; use workload identity or a service principal.

## Permissions

Grant access via RBAC:

```bash
az role assignment create \
  --role "Key Vault Secrets User" \
  --assignee "user@example.com" \
  --scope "/subscriptions/SUB-ID/resourceGroups/myapp-rg/providers/Microsoft.KeyVault/vaults/myapp-vault"
```

## Configuration

```toml
[providers]
azure = { type = "azure-sm", vault_url = "https://myapp-vault.vault.azure.net/", prefix = "myapp-" }  # prefix is optional
```

## Usage notes

The config stores a secret name. Keep the configured prefix consistent with the actual name in Key Vault, and refresh any fnox cache after a value changes.

## Next steps

- [Azure Key Vault Keys](/providers/azure-kms) - Encryption alternative
- [AWS Secrets Manager](/providers/aws-sm) - AWS equivalent
