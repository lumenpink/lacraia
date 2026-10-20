use lacraia_shared::updater::{
    can_update, format_auto_update_notice, format_startup_notice, get_default_config_path,
    get_default_state_path, get_distribution_endpoints, is_cache_expired, load_config,
    resolve_channel, save_config, update_config_pin, update_config_set_auto_check,
    update_config_set_auto_update, update_config_set_channel, update_config_unpin, LacraiaConfig,
    UpdateConfig, UpdateState,
};
use tempfile::tempdir;

#[test]
fn test_xdg_paths_resolution() {
    let temp = tempdir().unwrap();
    let xdg_config = temp.path().join("fake_xdg_config");
    let xdg_state = temp.path().join("fake_xdg_state");

    std::env::set_var("XDG_CONFIG_HOME", &xdg_config);
    std::env::set_var("XDG_STATE_HOME", &xdg_state);

    let config_path = get_default_config_path();
    let state_path = get_default_state_path();

    assert_eq!(config_path, xdg_config.join("lacraia").join("config.toml"));
    assert_eq!(
        state_path,
        xdg_state.join("lacraia").join("update_check.json")
    );

    std::env::remove_var("XDG_CONFIG_HOME");
    std::env::remove_var("XDG_STATE_HOME");
}

#[test]
fn test_config_defaults_and_serialization() {
    let temp = tempdir().unwrap();
    let config_file = temp.path().join("config.toml");

    let cfg = load_config(Some(&config_file));
    let update = cfg.update.expect("Update section should exist by default");
    assert_eq!(update.channel, "stable");
    assert!(!update.pinned);
    assert_eq!(update.pin, "");
    assert!(update.auto_check);
    assert_eq!(update.check_interval_hours, 24);
    assert!(!update.auto_update);

    // Save and reload
    let mut custom_cfg = LacraiaConfig::default();
    custom_cfg.update = Some(UpdateConfig {
        channel: "nightly".to_string(),
        pinned: true,
        pin: "0.2.0".to_string(),
        auto_check: false,
        check_interval_hours: 12,
        auto_update: true,
    });

    save_config(&custom_cfg, Some(&config_file)).unwrap();
    let reloaded = load_config(Some(&config_file));
    let r_update = reloaded.update.unwrap();
    assert_eq!(r_update.channel, "nightly");
    assert!(r_update.pinned);
    assert_eq!(r_update.pin, "0.2.0");
    assert!(!r_update.auto_check);
    assert_eq!(r_update.check_interval_hours, 12);
    assert!(r_update.auto_update);
}

#[test]
fn test_config_mutation_helpers() {
    let temp = tempdir().unwrap();
    let config_file = temp.path().join("config.toml");

    update_config_set_channel("nightly", Some(&config_file)).unwrap();
    assert_eq!(
        load_config(Some(&config_file)).update.unwrap().channel,
        "nightly"
    );

    update_config_pin("0.1.99", Some(&config_file)).unwrap();
    let pinned_cfg = load_config(Some(&config_file)).update.unwrap();
    assert!(pinned_cfg.pinned);
    assert_eq!(pinned_cfg.pin, "0.1.99");

    update_config_unpin(Some(&config_file)).unwrap();
    let unpinned_cfg = load_config(Some(&config_file)).update.unwrap();
    assert!(!unpinned_cfg.pinned);
    assert_eq!(unpinned_cfg.pin, "");

    update_config_set_auto_check(false, Some(&config_file)).unwrap();
    assert!(!load_config(Some(&config_file)).update.unwrap().auto_check);

    update_config_set_auto_update(true, Some(&config_file)).unwrap();
    assert!(load_config(Some(&config_file)).update.unwrap().auto_update);
}

#[test]
fn test_channel_resolution_hierarchy() {
    let temp = tempdir().unwrap();
    let config_file = temp.path().join("config.toml");

    let mut cfg = LacraiaConfig::default();
    cfg.update = Some(UpdateConfig {
        channel: "config_chan".to_string(),
        ..Default::default()
    });
    save_config(&cfg, Some(&config_file)).unwrap();

    // 1. Fallback / config
    std::env::remove_var("LACRAIA_CHANNEL");
    assert_eq!(
        resolve_channel(None, None, Some(&config_file)),
        "config_chan"
    );

    // 2. Env overrides config
    std::env::set_var("LACRAIA_CHANNEL", "env_chan");
    assert_eq!(resolve_channel(None, None, Some(&config_file)), "env_chan");

    // 3. Flag overrides everything
    assert_eq!(
        resolve_channel(Some("flag_chan"), None, Some(&config_file)),
        "flag_chan"
    );

    std::env::remove_var("LACRAIA_CHANNEL");
}

#[test]
fn test_pinning_guard() {
    let mut cfg = UpdateConfig {
        pinned: true,
        pin: "0.1.5".to_string(),
        ..Default::default()
    };

    assert!(can_update(&cfg, false).is_err());
    assert!(can_update(&cfg, true).is_ok());

    cfg.pinned = false;
    cfg.pin = "".to_string();
    assert!(can_update(&cfg, false).is_ok());
}

#[test]
fn test_cache_expiration_and_state() {
    let state = UpdateState {
        last_check_epoch: 1000,
        latest_version: "0.2.0".to_string(),
        channel: "stable".to_string(),
    };

    // 24 hours = 86400 seconds
    assert!(!is_cache_expired(&state, 24, 1000 + 3600)); // only 1 hour later
    assert!(is_cache_expired(&state, 24, 1000 + 86401)); // after 24 hours
}

#[test]
fn test_banca_endpoints() {
    let (stable, nightly, ver, n_ver, manifest) = get_distribution_endpoints();
    assert_eq!(
        stable,
        "https://banca.2lp.in/downloads/lacraia/stable-latest/lacraia"
    );
    assert_eq!(
        nightly,
        "https://banca.2lp.in/downloads/lacraia/nightly-latest/lacraia"
    );
    assert_eq!(ver, "https://banca.2lp.in/downloads/lacraia/version");
    assert_eq!(
        n_ver,
        "https://banca.2lp.in/downloads/lacraia/nightly-version"
    );
    assert_eq!(
        manifest,
        "https://banca.2lp.in/downloads/lacraia/index.json"
    );
}

#[test]
fn test_notice_formatting() {
    let startup_notice = format_startup_notice("lacraia", "stable", "0.1.0", "0.2.0");
    assert!(startup_notice
        .contains("💡 [lacraia] Nova versão disponível no canal stable: v0.1.0 -> v0.2.0"));
    assert!(startup_notice.contains("Execute 'lacraia update' para atualizar"));

    let auto_notice = format_auto_update_notice("lacraia", "0.1.0", "0.2.0");
    assert!(auto_notice.contains("✨ [lacraia] Atualização automática aplicada: v0.1.0 -> v0.2.0"));
    assert!(auto_notice.contains("A nova versão estará ativa na sua próxima execução"));
}
