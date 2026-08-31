#[macro_export]
macro_rules! send_telemetry_sync_from_ctx {
    ($event:expr_2021, $ctx:expr_2021) => {{
        let _ = (&$event, &$ctx);
    }};
}

#[macro_export]
macro_rules! send_telemetry_sync_from_app_ctx {
    ($event:expr_2021, $app_ctx:expr_2021) => {{
        let _ = (&$event, &$app_ctx);
    }};
}

#[macro_export]
macro_rules! send_telemetry_on_executor {
    ($auth_state:expr_2021, $event:expr_2021, $executor:expr_2021) => {{
        let _ = (&$auth_state, &$event, &$executor);
    }};
}
