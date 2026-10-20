use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

fn default_channel() -> String {
    "stable".to_string()
}

const fn default_auto_check() -> bool {
    true
}

const fn default_interval() -> u64 {
    24
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub pin: String,
    #[serde(default = "default_auto_check")]
    pub auto_check: bool,
    #[serde(default = "default_interval")]
    pub check_interval_hours: u64,
    #[serde(default)]
    pub auto_update: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: default_channel(),
            pinned: false,
            pin: String::new(),
            auto_check: default_auto_check(),
            check_interval_hours: default_interval(),
            auto_update: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LacraiaConfig {
    pub update: Option<UpdateConfig>,
}

impl Default for LacraiaConfig {
    fn default() -> Self {
        Self {
            update: Some(UpdateConfig::default()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct UpdateState {
    pub last_check_epoch: u64,
    pub latest_version: String,
    pub channel: String,
}

/// Retorna o caminho canônico do arquivo de configuração segundo o padrão XDG
pub fn get_default_config_path() -> PathBuf {
    if let Some(xdg) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return PathBuf::from(xdg.trim())
            .join("lacraia")
            .join("config.toml");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("lacraia")
            .join("config.toml");
    }
    PathBuf::from(".lacraia_config.toml")
}

/// Retorna o caminho canônico do arquivo de estado/cache segundo o padrão XDG
pub fn get_default_state_path() -> PathBuf {
    if let Some(xdg) = std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return PathBuf::from(xdg.trim())
            .join("lacraia")
            .join("update_check.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("lacraia")
            .join("update_check.json");
    }
    PathBuf::from(".lacraia_update_check.json")
}

/// Carrega o arquivo de configuração persistido
pub fn load_config(path: Option<&Path>) -> LacraiaConfig {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    if !p.exists() {
        return LacraiaConfig::default();
    }
    let mut cfg: LacraiaConfig = match fs::read_to_string(&p) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => LacraiaConfig::default(),
    };
    if cfg.update.is_none() {
        cfg.update = Some(UpdateConfig::default());
    }
    cfg
}

/// Salva o arquivo de configuração
pub fn save_config(
    cfg: &LacraiaConfig,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_config_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let toml_str = toml::to_string_pretty(cfg)?;
    fs::write(&p, toml_str)?;
    Ok(())
}

/// Define o canal ativo no arquivo de configuração
pub fn update_config_set_channel(
    channel: &str,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config(path);
    let mut update = cfg.update.unwrap_or_default();
    update.channel = channel.trim().to_lowercase();
    cfg.update = Some(update);
    save_config(&cfg, path)
}

/// Fixa uma versão no arquivo de configuração (pinning)
pub fn update_config_pin(
    version: &str,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config(path);
    let mut update = cfg.update.unwrap_or_default();
    update.pinned = true;
    update.pin = version.trim().trim_start_matches('v').to_string();
    cfg.update = Some(update);
    save_config(&cfg, path)
}

/// Remove o travamento de versão (unpin)
pub fn update_config_unpin(path: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config(path);
    let mut update = cfg.update.unwrap_or_default();
    update.pinned = false;
    update.pin = String::new();
    cfg.update = Some(update);
    save_config(&cfg, path)
}

/// Ajusta a checagem automática no arranque
pub fn update_config_set_auto_check(
    val: bool,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config(path);
    let mut update = cfg.update.unwrap_or_default();
    update.auto_check = val;
    cfg.update = Some(update);
    save_config(&cfg, path)
}

/// Ajusta a auto-atualização transparente
pub fn update_config_set_auto_update(
    val: bool,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = load_config(path);
    let mut update = cfg.update.unwrap_or_default();
    update.auto_update = val;
    cfg.update = Some(update);
    save_config(&cfg, path)
}

/// Resolve o canal ativo seguindo a precedência: Flag > Env > Config > "stable"
pub fn resolve_channel(
    flag: Option<&str>,
    env_val: Option<&str>,
    config_path: Option<&Path>,
) -> String {
    if let Some(f) = flag.map(str::trim).filter(|s| !s.is_empty()) {
        return f.to_lowercase();
    }
    if let Some(e) = env_val.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_lowercase();
    }
    if let Ok(e) = std::env::var("LACRAIA_CHANNEL") {
        let trimmed = e.trim();
        if !trimmed.is_empty() {
            return trimmed.to_lowercase();
        }
    }
    let cfg = load_config(config_path);
    if let Some(u) = cfg.update {
        let ch = u.channel.trim();
        if !ch.is_empty() {
            return ch.to_lowercase();
        }
    }
    "stable".to_string()
}

/// Verifica se a atualização é permitida com base no estado de pinning
pub fn can_update(cfg: &UpdateConfig, force: bool) -> Result<(), String> {
    if (cfg.pinned || !cfg.pin.is_empty()) && !force {
        let pinned_ver = if cfg.pin.is_empty() {
            "travada".to_string()
        } else {
            format!("fixada em v{}", cfg.pin)
        };
        return Err(format!(
            "Atualização bloqueada: versão {} (pin). Use --force para sobrescrever.",
            pinned_ver
        ));
    }
    Ok(())
}

/// Valida se o cache de verificação de versão expirou
pub fn is_cache_expired(state: &UpdateState, interval_hours: u64, current_epoch: u64) -> bool {
    let threshold_secs = interval_hours.saturating_mul(3600);
    current_epoch.saturating_sub(state.last_check_epoch) >= threshold_secs
}

/// Carrega o estado de verificação persistido
pub fn load_state(path: Option<&Path>) -> Option<UpdateState> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_state_path);
    if !p.exists() {
        return None;
    }
    let content = fs::read_to_string(&p).ok()?;
    serde_json::from_str(&content).ok()
}

/// Salva o estado de verificação persistido
pub fn save_state(
    state: &UpdateState,
    path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let p = path
        .map(PathBuf::from)
        .unwrap_or_else(get_default_state_path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let json_str = serde_json::to_string_pretty(state)?;
    fs::write(&p, json_str)?;
    Ok(())
}

/// Retorna as URLs oficiais de distribuição soberana na Banca
pub fn get_distribution_endpoints() -> (String, String, String, String, String) {
    (
        "https://banca.2lp.in/downloads/lacraia/stable-latest/lacraia".to_string(),
        "https://banca.2lp.in/downloads/lacraia/nightly-latest/lacraia".to_string(),
        "https://banca.2lp.in/downloads/lacraia/version".to_string(),
        "https://banca.2lp.in/downloads/lacraia/nightly-version".to_string(),
        "https://banca.2lp.in/downloads/lacraia/index.json".to_string(),
    )
}

/// Formata a mensagem de aviso não-bloqueante no arranque
pub fn format_startup_notice(app: &str, channel: &str, current: &str, remote: &str) -> String {
    format!(
        "💡 [{}] Nova versão disponível no canal {}: v{} -> v{}\n   Execute '{} update' para atualizar.",
        app, channel, current, remote, app
    )
}

/// Formata a mensagem de notificação de auto-atualização transparente
pub fn format_auto_update_notice(app: &str, current: &str, remote: &str) -> String {
    format!(
        "✨ [{}] Atualização automática aplicada: v{} -> v{}.\n   A nova versão estará ativa na sua próxima execução.",
        app, current, remote
    )
}
