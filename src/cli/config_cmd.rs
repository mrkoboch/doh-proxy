use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};

use doh_proxy::config::{CacheConfig, Config, config_path};

pub fn run() -> anyhow::Result<()> {
    let theme = ColorfulTheme::default();
    let current = Config::load_or_create()?;

    println!(
        "{} Interactive config setup",
        style("◆").cyan().bold()
    );
    println!(
        "  Config path: {}\n",
        style(config_path().display().to_string()).dim()
    );

    // Listen address
    let listen_addr: String = Input::with_theme(&theme)
        .with_prompt("Listen address")
        .default(current.listen_addr.to_string())
        .validate_with(|s: &String| {
            s.parse::<std::net::SocketAddr>()
                .map(|_| ())
                .map_err(|e| format!("invalid socket address: {e}"))
        })
        .interact_text()?;

    // Upstream resolvers — edit existing ones first
    println!(
        "\n  {}",
        style("Upstream DoH resolvers (enter one per line; empty line to finish):").dim()
    );
    let mut upstreams: Vec<String> = Vec::new();
    for (i, default) in current.upstreams.iter().enumerate() {
        let prompt = format!("Upstream {}", i + 1);
        let val: String = Input::with_theme(&theme)
            .with_prompt(&prompt)
            .default(default.clone())
            .allow_empty(true)
            .interact_text()?;
        if val.is_empty() {
            break;
        }
        upstreams.push(val);
    }
    // Allow adding more upstreams beyond the current list
    let mut idx = current.upstreams.len() + 1;
    loop {
        let val: String = Input::with_theme(&theme)
            .with_prompt(format!("Upstream {idx} (empty to finish)"))
            .allow_empty(true)
            .interact_text()?;
        if val.is_empty() {
            break;
        }
        upstreams.push(val);
        idx += 1;
    }
    if upstreams.is_empty() {
        upstreams = Config::default().upstreams;
        println!(
            "  {} No upstreams entered — using defaults.",
            style("!").yellow()
        );
    }

    // Cache settings
    let cache_enabled: bool = Confirm::with_theme(&theme)
        .with_prompt("Enable DNS response cache?")
        .default(current.cache.enabled)
        .interact()?;

    let cache_capacity: u64 = Input::with_theme(&theme)
        .with_prompt("Cache capacity (max entries)")
        .default(current.cache.capacity)
        .validate_with(|s: &u64| {
            if *s == 0 {
                Err("capacity must be > 0".to_string())
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let config = Config {
        listen_addr: listen_addr.parse()?,
        upstreams,
        cache: CacheConfig {
            enabled: cache_enabled,
            capacity: cache_capacity,
        },
    };

    config.save()?;

    println!(
        "\n{} Config saved to {}",
        style("✓").green(),
        style(config_path().display().to_string()).cyan()
    );
    println!("  Run `doh-proxy start` to apply the new configuration.");

    Ok(())
}
