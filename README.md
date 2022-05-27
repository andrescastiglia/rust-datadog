# Datadog APM for Rust (original fork from datadog-apm)

Credits
-------
Forked from <https://github.com/pipefy/datadog-apm-rust>.

Usage
------

### Config

```rust
{
    let enabled = true;
    let config = Config::new(
        "service_name".to_string(),
        Some("staging".to_string()),
        "localhost:8196".to_string(),
        LoggingConfig {
            level: Level::Debug,
            ..LoggingConfig::default()
        },
        ApmConfig::default(),
        enabled,
    );
    let _client = DatadogTracing::new(config);

```

### Instrumentation

```rust
#[tracing::instrument]
pub fn foo(name: &str) {
    debug!("Hello, {}!", name);
}
```

### Span

```rust
{
    let span = span!(Level::INFO, "foo");
    let _enter = span.enter();
    info!("greeting");
}
```

More
------

See also [`tracing`](https://crates.io/crates/tracing)