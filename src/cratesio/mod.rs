use anyhow::{Context, Result};
use serde::Deserialize;

use crate::version::Version;

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    krate: CratesIoCrate,
    #[serde(default)]
    versions: Vec<CratesIoVersion>,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    max_stable_version: Option<String>,
    max_version: String,
}

#[derive(Debug, Deserialize)]
struct CratesIoVersion {
    num: String,
}

pub fn latest_published_version(crate_name: &str) -> Result<Option<Version>> {
    let response = fetch_crate(crate_name)?;
    let version = response
        .krate
        .max_stable_version
        .as_deref()
        .unwrap_or(&response.krate.max_version);
    Ok(version.parse().ok())
}

pub fn has_version(crate_name: &str, version: &Version) -> Result<bool> {
    let response = fetch_crate(crate_name)?;
    Ok(response
        .versions
        .iter()
        .any(|release| release.num == version.to_string()))
}

fn fetch_crate(crate_name: &str) -> Result<CratesIoResponse> {
    fetch_crate_from_base("https://crates.io", crate_name)
}

fn fetch_crate_from_base(base_url: &str, crate_name: &str) -> Result<CratesIoResponse> {
    let url = format!(
        "{}/api/v1/crates/{}",
        base_url.trim_end_matches('/'),
        crate_name
    );
    let response = ureq::get(&url)
        .set("User-Agent", "relx")
        .call()
        .with_context(|| format!("failed to fetch crates.io metadata for {crate_name}"))?;
    let raw: serde_json::Value = response
        .into_json()
        .with_context(|| format!("failed to parse crates.io metadata for {crate_name}"))?;

    let payload = serde_json::json!({
        "krate": raw.get("crate"),
        "versions": raw.get("versions").cloned().unwrap_or_else(|| serde_json::json!([])),
    });

    serde_json::from_value(payload)
        .with_context(|| format!("failed to decode crates.io payload for {crate_name}"))
}

#[cfg(test)]
mod tests {
    use super::fetch_crate_from_base;
    use crate::version::Version;
    use mockito::Server;

    #[test]
    fn latest_published_version_prefers_stable() {
        let mut server = Server::new();
        server
            .mock("GET", "/api/v1/crates/demo")
            .with_body(
                r#"{
                    "crate": {
                        "id": "demo",
                        "max_version": "1.2.0-rc.1",
                        "max_stable_version": "1.1.0"
                    },
                    "versions": [
                        {"num": "1.1.0"},
                        {"num": "1.2.0-rc.1"}
                    ]
                }"#,
            )
            .create();

        let version = fetch_crate_from_base(&server.url(), "demo")
            .expect("fetch crate")
            .krate
            .max_stable_version;
        assert_eq!(version.as_deref(), Some("1.1.0"));

        let parsed = latest_published_version_from(&server.url(), "demo").expect("latest version");
        assert_eq!(parsed.expect("version").to_string(), "1.1.0");
    }

    #[test]
    fn has_version_matches_exact_release() {
        let mut server = Server::new();
        server
            .mock("GET", "/api/v1/crates/demo")
            .with_body(
                r#"{
                    "crate": {
                        "id": "demo",
                        "max_version": "0.3.0",
                        "max_stable_version": "0.3.0"
                    },
                    "versions": [
                        {"num": "0.2.0"},
                        {"num": "0.3.0"}
                    ]
                }"#,
            )
            .create();

        assert!(has_version_from(&server.url(), "demo", "0.3.0"));
        assert!(!has_version_from(&server.url(), "demo", "0.4.0"));
    }

    fn latest_published_version_from(
        base: &str,
        crate_name: &str,
    ) -> Result<Option<Version>, anyhow::Error> {
        let response = fetch_crate_from_base(base, crate_name)?;
        let version = response
            .krate
            .max_stable_version
            .as_deref()
            .unwrap_or(&response.krate.max_version);
        Ok(version.parse().ok())
    }

    fn has_version_from(base: &str, crate_name: &str, version: &str) -> bool {
        let response = fetch_crate_from_base(base, crate_name).expect("fetch crate");
        response
            .versions
            .iter()
            .any(|release| release.num == version)
    }
}
