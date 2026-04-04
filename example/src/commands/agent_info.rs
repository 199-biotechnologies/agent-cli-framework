/// Machine-readable capability manifest.
///
/// agent-info is always JSON -- the whole point is machine readability.
/// It uses its own schema (not the envelope) because it IS the schema
/// definition. An agent calling agent-info is bootstrapping.

pub fn run() {
    let info = serde_json::json!({
        "name": "greeter",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Minimal agent-friendly CLI example",
        "commands": {
            "hello <name>": "Greet someone. Styles: friendly, formal, pirate.",
            "agent-info | info": "This manifest.",
            "skill install": "Install skill file to agent platforms.",
            "skill status": "Check skill installation status.",
            "update [--check]": "Self-update binary from GitHub Releases.",
        },
        "flags": {
            "--json": "Force JSON output (auto-enabled when piped)",
            "--style": "Greeting style: friendly, formal, pirate",
        },
        "exit_codes": {
            "0": "Success",
            "1": "Transient error (IO, network) -- retry",
            "2": "Config error -- fix setup",
            "3": "Bad input -- fix arguments",
            "4": "Rate limited -- wait and retry",
        },
        "envelope": {
            "version": "1",
            "success": "{ version, status, data }",
            "error": "{ version, status, error: { code, message, suggestion } }",
        },
        "auto_json_when_piped": true,
    });
    println!("{}", serde_json::to_string_pretty(&info).unwrap());
}
