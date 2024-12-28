// macros.rs

#[macro_export]
macro_rules! init_logging {
    () => {
        use tracing_subscriber::{fmt, EnvFilter};
        
        fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| {
                        EnvFilter::new(
                            "info,reqwest=info,hyper=off,h2=off,rustls=off,tungstenite=info"
                        )
                    })
            )
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .init();
    };
}