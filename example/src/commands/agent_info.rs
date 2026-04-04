/// Machine-readable capability manifest.
///
/// agent-info is always JSON -- the whole point is machine readability.
/// It uses its own schema (not the envelope) because it IS the schema
/// definition. An agent calling agent-info is bootstrapping.
pub fn run() {
    let name = env!("CARGO_PKG_NAME");
    let config_path = crate::config::config_path();

    let info = serde_json::json!({
        "name": name,
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "commands": {
            "hello": {
                "description": "Greet someone",
                "args": [
                    {
                        "name": "name",
                        "kind": "positional",
                        "type": "string",
                        "required": true,
                        "description": "Name to greet"
                    }
                ],
                "options": [
                    {
                        "name": "--style",
                        "type": "string",
                        "required": false,
                        "default": "friendly",
                        "values": ["friendly", "formal", "pirate"],
                        "description": "Greeting style"
                    }
                ]
            },
            "agent-info": {
                "description": "This manifest",
                "aliases": ["info"],
                "args": [],
                "options": []
            },
            "skill install": {
                "description": "Install skill file to agent platforms",
                "args": [],
                "options": []
            },
            "skill status": {
                "description": "Check skill installation status",
                "args": [],
                "options": []
            },
            "config show": {
                "description": "Display effective merged configuration",
                "args": [],
                "options": []
            },
            "config path": {
                "description": "Show configuration file path",
                "args": [],
                "options": []
            },
            "update": {
                "description": "Self-update binary from GitHub Releases",
                "args": [],
                "options": [
                    {
                        "name": "--check",
                        "type": "bool",
                        "required": false,
                        "default": false,
                        "description": "Check only, don't install"
                    }
                ]
            }
        },
        "global_flags": {
            "--json": {
                "description": "Force JSON output (auto-enabled when piped)",
                "type": "bool",
                "default": false
            },
            "--quiet": {
                "description": "Suppress informational output",
                "type": "bool",
                "default": false
            }
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
        "config": {
            "path": config_path.display().to_string(),
            "env_prefix": format!("{}_", name.to_uppercase()),
        },
        "auto_json_when_piped": true,
    });
    println!("{}", serde_json::to_string_pretty(&info).unwrap());
}
