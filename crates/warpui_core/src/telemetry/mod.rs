// Modified for term4u: telemetry transport removed; this module is a no-op shim.
// Original: Copyright (C) 2020-2026 Denver Technologies, Inc. (MIT, see /LICENSE-MIT)

#[macro_export]
macro_rules! record_telemetry_from_ctx {
    ($user_id:expr, $anonymous_id:expr, $name:expr, $payload:expr, $contains_ugc:expr, $ctx:expr) => {{
        let _ = (
            &$user_id,
            &$anonymous_id,
            &$name,
            &$payload,
            &$contains_ugc,
            &$ctx,
        );
    }};
}

#[macro_export]
macro_rules! record_telemetry_on_executor {
    ($user_id:expr, $anonymous_id:expr, $name:expr, $payload:expr, $contains_ugc:expr, $executor:expr) => {{
        let _ = (
            &$user_id,
            &$anonymous_id,
            &$name,
            &$payload,
            &$contains_ugc,
            &$executor,
        );
    }};
}
