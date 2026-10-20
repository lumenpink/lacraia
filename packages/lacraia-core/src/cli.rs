use lacraia_shared::updater::{
    can_update, format_auto_update_notice, format_startup_notice, get_default_config_path,
    get_distribution_endpoints, is_cache_expired, load_config, load_state, resolve_channel,
    save_state, update_config_pin, update_config_set_auto_check, update_config_set_auto_update,
    update_config_set_channel, update_config_unpin, UpdateState,
};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_help() {
    println!("La Craia — Automotive audio streaming engine & sovereign player\n");
    println!("USO:");
    println!("  lacraia-core [COMANDO] [OPÇÕES]\n");
    println!("COMANDOS:");
    println!("  check-update                 Verifica se há novas versões disponíveis na Banca");
    println!("  update                       Atualiza o binário soberano de forma atômica");
    println!("  config <SUBCOMANDO>          Gerencia opções de configuração XDG (config.toml)");
    println!("\nSUBCOMANDOS DE CONFIGURAÇÃO:");
    println!("  config show                  Exibe a configuração atual em formato TOML");
    println!("  config get <CHAVE>           Exibe o valor de uma chave específica");
    println!("  config pin [<VERSÃO>]        Trava a versão atual ou especificada");
    println!("  config unpin                 Remove o travamento de versão");
    println!("  config set-channel <CANAL>   Altera o canal padrão ('stable' ou 'nightly')");
    println!("  config set-auto-check <B>    Habilita/desabilita checagem automática (true|false)");
    println!("  config set-auto-update <B>   Habilita/desabilita auto-atualização transparente (true|false)");
    println!("\nOPÇÕES:");
    println!("  -h, --help                   Exibe esta ajuda");
    println!("  -V, --version                Exibe a versão instalada");
}

pub fn semver_is_newer(remote: &str, current: &str) -> bool {
    let r_clean = remote.trim().trim_start_matches('v');
    let c_clean = current.trim().trim_start_matches('v');

    if r_clean == c_clean {
        return false;
    }

    if let (Ok(r_ver), Ok(c_ver)) = (
        semver::Version::parse(r_clean),
        semver::Version::parse(c_clean),
    ) {
        return r_ver > c_ver;
    }

    r_clean > c_clean
}

pub async fn cmd_check_update(channel_opt: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let channel = resolve_channel(channel_opt, None, None);
    let (_, _, ver_url, n_ver_url, _) = get_distribution_endpoints();
    let target_url = if channel == "nightly" {
        n_ver_url
    } else {
        ver_url
    };

    println!("⚡ La Craia Updater // Banca Soberana 2lp.in");
    println!("Canal de Distribuição : {}", channel);
    println!("Versão Local Atual    : v{}", CURRENT_VERSION);
    println!("Endpoint de Consulta  : {}", target_url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()?;

    let resp = match client.get(&target_url).send().await {
        Ok(r) if r.status().is_success() => r.text().await.unwrap_or_default().trim().to_string(),
        _ => String::new(),
    };

    if resp.is_empty() {
        println!("Versão Remota         : indisponível (servidor offline ou rota não publicada)");
        println!("\nNão foi possível verificar atualizações no momento.");
        return Ok(());
    }

    println!("Versão Remota         : v{}", resp);

    if semver_is_newer(&resp, CURRENT_VERSION) {
        println!(
            "\n★ Nova versão disponível: v{} -> v{}",
            CURRENT_VERSION, resp
        );
        println!("Para atualizar agora, execute:");
        println!("  lacraia-core update --channel {}", channel);
    } else {
        println!(
            "\n✔ O La Craia já está atualizado na versão mais recente do canal {}.",
            channel
        );
    }

    Ok(())
}

pub fn get_download_url_for_channel(channel: &str, pin: Option<&str>) -> String {
    if let Some(p) = pin {
        let clean = p.trim().trim_start_matches('v');
        format!(
            "https://banca.2lp.in/downloads/lacraia/releases/v{}/lacraia-core",
            clean
        )
    } else if channel == "nightly" {
        "https://banca.2lp.in/downloads/lacraia/nightly-latest/lacraia-core".to_string()
    } else {
        "https://banca.2lp.in/downloads/lacraia/stable-latest/lacraia-core".to_string()
    }
}

pub async fn cmd_update(
    channel_opt: Option<&str>,
    pin_opt: Option<&str>,
    force: bool,
    target_override: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config(None);
    let update_cfg = cfg.update.unwrap_or_default();

    if let Err(msg) = can_update(&update_cfg, force) {
        eprintln!("✘ {}", msg);
        return Err(msg.into());
    }

    let channel = resolve_channel(channel_opt, None, None);
    let effective_pin = pin_opt.or(if !update_cfg.pin.is_empty() {
        Some(update_cfg.pin.as_str())
    } else {
        None
    });

    let download_url = get_download_url_for_channel(&channel, effective_pin);
    println!("⚡ La Craia Auto-Update");
    println!("Canal Selecionado : {}", channel);
    if let Some(p) = effective_pin {
        println!("Versão Fixada     : v{}", p);
    }
    println!("Versão Atual      : v{}", CURRENT_VERSION);
    println!("Baixando de       : {}", download_url);

    let target_path = if let Some(t) = target_override {
        t.to_path_buf()
    } else {
        std::env::current_exe()?
    };

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;

    let resp = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Falha de conexão com {}: {}", download_url, e))?;

    if !resp.status().is_success() {
        return Err(format!("Download falhou com HTTP status {}", resp.status()).into());
    }

    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        return Err("Arquivo baixado está vazio.".into());
    }

    let target_dir = target_path
        .parent()
        .ok_or("Não foi possível identificar o diretório do binário alvo.")?;

    let mut temp_file = tempfile::Builder::new()
        .prefix(".lacraia-update-")
        .tempfile_in(target_dir)?;

    temp_file.write_all(&bytes)?;
    temp_file.flush()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(temp_file.path(), perms)?;
    }

    temp_file.persist(&target_path)?;

    println!(
        "✔ La Craia atualizado com sucesso em '{}'!",
        target_path.display()
    );
    Ok(())
}

pub fn cmd_config(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let sub = args.first().map(String::as_str).unwrap_or("show");
    match sub {
        "show" => {
            let p = get_default_config_path();
            println!("# Arquivo: {}", p.display());
            if p.exists() {
                let content = fs::read_to_string(&p)?;
                println!("{}", content);
            } else {
                let cfg = load_config(None);
                let toml_str = toml::to_string_pretty(&cfg)?;
                println!("{}", toml_str);
            }
        }
        "get" => {
            let key = args.get(1).map(String::as_str).unwrap_or("");
            let cfg = load_config(None);
            let u = cfg.update.unwrap_or_default();
            match key {
                "channel" => println!("{}", u.channel),
                "pinned" => println!("{}", u.pinned),
                "pin" => println!("{}", u.pin),
                "auto_check" => println!("{}", u.auto_check),
                "check_interval_hours" => println!("{}", u.check_interval_hours),
                "auto_update" => println!("{}", u.auto_update),
                _ => {
                    eprintln!("Chave desconhecida: '{}'. Chaves válidas: channel, pinned, pin, auto_check, check_interval_hours, auto_update", key);
                }
            }
        }
        "pin" => {
            let ver = args.get(1).map(String::as_str).unwrap_or(CURRENT_VERSION);
            update_config_pin(ver, None)?;
            println!("✔ Versão fixada em v{}", ver.trim_start_matches('v'));
        }
        "unpin" => {
            update_config_unpin(None)?;
            println!("✔ Travamento de versão desativado.");
        }
        "set-channel" => {
            let ch = args.get(1).map(String::as_str).unwrap_or("stable");
            update_config_set_channel(ch, None)?;
            println!("✔ Canal configurado para '{}'.", ch);
        }
        "set-auto-check" => {
            let b = args.get(1).map(String::as_str).unwrap_or("true") == "true";
            update_config_set_auto_check(b, None)?;
            println!("✔ Checagem automática configurada para: {}.", b);
        }
        "set-auto-update" => {
            let b = args.get(1).map(String::as_str).unwrap_or("false") == "true";
            update_config_set_auto_update(b, None)?;
            println!("✔ Auto-atualização transparente configurada para: {}.", b);
        }
        _ => {
            eprintln!("Subcomando de configuração desconhecido: '{}'", sub);
            print_help();
        }
    }
    Ok(())
}

pub async fn handle_cli_args(args: &[String]) -> Result<bool, Box<dyn std::error::Error>> {
    if args.is_empty() {
        return Ok(false);
    }

    match args[0].as_str() {
        "-h" | "--help" | "help" => {
            print_help();
            Ok(true)
        }
        "-V" | "--version" | "version" => {
            println!("lacraia v{}", CURRENT_VERSION);
            Ok(true)
        }
        "check-update" => {
            let mut channel = None;
            for i in 1..args.len() {
                if args[i] == "--channel" && i + 1 < args.len() {
                    channel = Some(args[i + 1].as_str());
                }
            }
            cmd_check_update(channel).await?;
            Ok(true)
        }
        "update" => {
            let mut channel = None;
            let mut pin = None;
            let mut force = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--channel" if i + 1 < args.len() => {
                        channel = Some(args[i + 1].as_str());
                        i += 1;
                    }
                    "--pin" if i + 1 < args.len() => {
                        pin = Some(args[i + 1].as_str());
                        i += 1;
                    }
                    "--force" => force = true,
                    _ => {}
                }
                i += 1;
            }
            cmd_update(channel, pin, force, None).await?;
            Ok(true)
        }
        "config" => {
            cmd_config(&args[1..])?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn spawn_startup_check() {
    tokio::spawn(async move {
        let cfg = load_config(None);
        let update_cfg = match cfg.update {
            Some(u) if u.auto_check => u,
            _ => return,
        };

        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let cached_state = load_state(None);
        if let Some(ref st) = cached_state {
            if !is_cache_expired(st, update_cfg.check_interval_hours, now_epoch) {
                return;
            }
        }

        let channel = resolve_channel(None, None, None);
        let (_, _, ver_url, n_ver_url, _) = get_distribution_endpoints();
        let target_url = if channel == "nightly" {
            n_ver_url
        } else {
            ver_url
        };

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
        {
            Ok(c) => c,
            Err(_) => return,
        };

        let remote_ver = match client.get(&target_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.text().await.unwrap_or_default().trim().to_string()
            }
            _ => return,
        };

        if remote_ver.is_empty() {
            return;
        }

        let new_state = UpdateState {
            last_check_epoch: now_epoch,
            latest_version: remote_ver.clone(),
            channel: channel.clone(),
        };
        let _ = save_state(&new_state, None);

        if semver_is_newer(&remote_ver, CURRENT_VERSION) {
            if update_cfg.auto_update && !update_cfg.pinned && update_cfg.pin.is_empty() {
                // Executa auto-update transparente
                if cmd_update(Some(&channel), None, false, None).await.is_ok() {
                    eprintln!(
                        "{}",
                        format_auto_update_notice("lacraia", CURRENT_VERSION, &remote_ver)
                    );
                }
            } else if !update_cfg.pinned && update_cfg.pin.is_empty() {
                eprintln!(
                    "{}",
                    format_startup_notice("lacraia", &channel, CURRENT_VERSION, &remote_ver)
                );
            }
        }
    });
}
