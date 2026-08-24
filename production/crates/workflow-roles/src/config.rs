use std::{
    collections::BTreeMap,
    path::Path,
    {fs, path::PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const READ_ONLY_ROLES: [&str; 4] = [
    "architect",
    "functional_reviewer",
    "security_reviewer",
    "arbiter",
];

const CONFIG_DIRECTORY: &str = "config";
const ROLES_FILE: &str = "roles.json";

#[derive(Debug)]
pub enum RolesError {
    Invalid(Vec<String>),
    Io(std::io::Error),
    Missing,
}

impl std::fmt::Display for RolesError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(issues) => {
                write!(formatter, "{}", issues.join("; "))
            }
            Self::Io(error) => write!(formatter, "roles.json cannot be read: {error}"),
            Self::Missing => write!(
                formatter,
                "config/roles.json is missing; configure the four read-only role models first"
            ),
        }
    }
}

impl std::error::Error for RolesError {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RolesFile {
    pub version: u8,
    #[serde(default)]
    pub roles: BTreeMap<String, RoleEndpoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RoleEndpoint {
    #[serde(default)]
    pub api_format: Option<String>,
    pub base_url: String,
    pub model_id: String,
    pub api_key_env: Option<String>,
    pub api_key_file: Option<String>,
}

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CONFIG_DIRECTORY).join(ROLES_FILE)
}

pub fn load(data_dir: &Path) -> Result<RolesFile, RolesError> {
    let path = config_path(data_dir);
    if !path.is_file() {
        return Err(RolesError::Missing);
    }
    let content = fs::read_to_string(&path).map_err(RolesError::Io)?;
    let file: RolesFile = serde_json::from_str(&content)
        .map_err(|error| RolesError::Invalid(vec![format!("roles.json is invalid: {error}")]))?;
    file.validate(data_dir).map_err(RolesError::Invalid)?;
    Ok(file)
}

impl RolesFile {
    pub fn validate(&self, data_dir: &Path) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();
        if self.version != 1 {
            issues.push(format!("version must be 1, found {}", self.version));
        }
        for role in READ_ONLY_ROLES {
            match self.roles.get(role) {
                None => issues.push(format!("role '{role}' is missing")),
                Some(endpoint) => issues.extend(endpoint.validate(role, data_dir)),
            }
        }
        for role in self.roles.keys() {
            if !READ_ONLY_ROLES.contains(&role.as_str()) {
                issues.push(format!("unknown role '{role}'"));
            }
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    #[must_use]
    pub fn endpoint(&self, role: &str) -> Option<&RoleEndpoint> {
        self.roles.get(role)
    }

    pub fn report(&self) -> Value {
        let roles = self
            .roles
            .iter()
            .map(|(role, endpoint)| {
                let host = endpoint.base_url_host();
                (
                    role.clone(),
                    json!({
                        "baseUrl": host,
                        "configured": true,
                        "keySource": endpoint.key_source(),
                        "modelId": endpoint.model_id,
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        json!({
            "executor": {
                "configured": true,
                "note": "the executor uses the model selected in Trae Work",
            },
            "readOnlyRoles": roles,
        })
    }
}

impl RoleEndpoint {
    fn validate(&self, role: &str, data_dir: &Path) -> Vec<String> {
        let mut issues = Vec::new();
        if let Some(format) = &self.api_format
            && format != "openai_compatible"
        {
            issues.push(format!(
                "role '{role}': api_format must be openai_compatible"
            ));
        }
        if self.base_url_host().is_none() {
            issues.push(format!("role '{role}': base_url must be an http(s) URL"));
        }
        if self.model_id.trim().is_empty() {
            issues.push(format!("role '{role}': model_id must not be empty"));
        }
        match (&self.api_key_env, &self.api_key_file) {
            (None, None) => issues.push(format!("role '{role}': set api_key_env or api_key_file")),
            (Some(_), Some(_)) => issues.push(format!(
                "role '{role}': api_key_env and api_key_file are mutually exclusive"
            )),
            (Some(name), None) => {
                if name.trim().is_empty() {
                    issues.push(format!("role '{role}': api_key_env must not be empty"));
                } else if std::env::var(name).is_err() {
                    issues.push(format!(
                        "role '{role}': environment variable {name} is not set"
                    ));
                }
            }
            (None, Some(relative)) => {
                let path = data_dir.join(relative);
                if !path.is_file() {
                    issues.push(format!(
                        "role '{role}': api key file {relative} does not exist"
                    ));
                }
            }
        }
        issues
    }

    #[must_use]
    pub fn base_url_host(&self) -> Option<String> {
        let rest = self
            .base_url
            .strip_prefix("https://")
            .or_else(|| self.base_url.strip_prefix("http://"))?;
        let host = rest.split(['/', '?']).next().unwrap_or_default();
        (!host.is_empty()).then(|| host.to_owned())
    }

    #[must_use]
    pub fn key_source(&self) -> Value {
        if let Some(name) = &self.api_key_env {
            json!(format!("env:{name}"))
        } else if let Some(file) = &self.api_key_file {
            json!(format!("file:{file}"))
        } else {
            Value::Null
        }
    }
}

pub fn missing_report() -> Value {
    let roles = READ_ONLY_ROLES
        .iter()
        .map(|role| {
            (
                (*role).to_owned(),
                json!({
                    "configured": false,
                    "keySource": null,
                    "modelId": null,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "configured": false,
        "path": CONFIG_DIRECTORY.to_owned() + "/" + ROLES_FILE,
        "readOnlyRoles": roles,
    })
}

#[cfg(test)]
mod tests {
    use super::{READ_ONLY_ROLES, RolesFile};
    use serde_json::json;

    fn endpoint_json(env: Option<&str>, file: Option<&str>) -> serde_json::Value {
        let mut value = json!({
            "base_url": "https://api.example.com/v1",
            "model_id": "provider/model",
        });
        if let Some(env) = env {
            value["api_key_env"] = json!(env);
        }
        if let Some(file) = file {
            value["api_key_file"] = json!(file);
        }
        value
    }

    fn file_json(env: Option<&str>, file: Option<&str>) -> RolesFile {
        let roles = READ_ONLY_ROLES
            .iter()
            .map(|role| ((*role).to_owned(), endpoint_json(env, file)))
            .collect::<serde_json::Map<_, _>>();
        serde_json::from_value(json!({
            "version": 1,
            "roles": roles,
        }))
        .unwrap()
    }

    #[test]
    fn valid_file_passes_with_environment_key() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_json(Some("PATH"), None);
        assert!(file.validate(directory.path()).is_ok());
    }

    #[test]
    fn missing_role_and_key_source_are_reported() {
        let directory = tempfile::tempdir().unwrap();
        let file = file_json(None, None);
        let issues = file.validate(directory.path()).unwrap_err();
        assert!(issues.len() == READ_ONLY_ROLES.len());
        assert!(issues.iter().all(|issue| issue.contains("api_key")));
    }

    #[test]
    fn non_http_base_url_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let mut value = serde_json::to_value(file_json(Some("PATH"), None)).unwrap();
        value["roles"]["arbiter"]["base_url"] = json!("ftp://example.com");
        let file: RolesFile = serde_json::from_value(value).unwrap();
        let issues = file.validate(directory.path()).unwrap_err();
        assert!(issues.len() == 1 && issues[0].contains("base_url"));
    }

    #[test]
    fn key_file_resolves_against_the_data_directory() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("arbiter.key"), b"k").unwrap();
        let mut value = serde_json::to_value(file_json(None, None)).unwrap();
        value["roles"]["arbiter"]["api_key_file"] = json!("arbiter.key");
        let file: RolesFile = serde_json::from_value(value).unwrap();
        let issues = file.validate(directory.path()).unwrap_err();
        assert!(issues.len() == 3 && issues.iter().all(|i| !i.contains("arbiter")));
    }

    #[test]
    fn endpoint_lookup_covers_configured_roles_only() {
        let file = file_json(Some("PATH"), None);
        assert!(file.endpoint("architect").is_some());
        assert!(file.endpoint("executor").is_none());
    }
}
