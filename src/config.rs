use std::{path::PathBuf, sync::OnceLock};

use anyhow::Result;
use kovi::{
    log::{info, warn},
    tokio::fs,
};
use openai::Credentials;
use serde::Deserialize;

pub(crate) static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Deserialize, Clone)]
pub(crate) struct RepoConfig {
    pub(crate) groups: Vec<i64>,
    pub(crate) time: String,
    pub(crate) interval: u32,
    pub(crate) owner: String,
    pub(crate) repo: String,
}

#[derive(Deserialize)]
struct LLMConfigFile {
    url: String,
    key: String,
    model: String,
    prompt_summary: String,
    prompt_criticize: String,
}

#[derive(Deserialize)]
struct ConfigFile {
    github_token: Option<String>,
    llm: LLMConfigFile,
    repos: Vec<RepoConfig>,
}

pub(crate) struct LLMConfig {
    pub(crate) cred: Credentials,
    pub(crate) model: String,
    pub(crate) prompt_summary: String,
    pub(crate) prompt_criticize: String,
}

pub(crate) struct Config {
    pub(crate) github_token: Option<String>,
    pub(crate) llm: LLMConfig,
    pub(crate) repos: Vec<RepoConfig>,
}

pub(crate) async fn init(path: PathBuf) -> Result<&'static Config> {
    let config_path = path.join("config.toml");

    let config_txt = match fs::read_to_string(&config_path).await {
        Ok(txt) => txt,
        Err(e) => {
            warn!("[octowatch] Failed to read config file: {e}");
            info!("[octowatch] Using default config");
            String::new()
        }
    };

    // .with_context(|| format!("Failed to read config file at {}", config_path.display()))?;
    let config: ConfigFile = toml::from_str(&config_txt)?;
    Ok(CONFIG.get_or_init(|| Config {
        github_token: config.github_token,
        llm: LLMConfig {
            cred: Credentials::new(config.llm.key, config.llm.url),
            model: config.llm.model,
            prompt_summary: config.llm.prompt_summary,
            prompt_criticize: config.llm.prompt_criticize,
        },
        repos: config.repos,
    }))
}
