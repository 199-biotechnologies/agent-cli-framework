#!/usr/bin/env bash
# Conformance probe for Agent CLI Framework binaries.
#
# Usage: conformance/conformance.sh <path-to-binary>
#
# Probes a built binary against the behavioral contract in README/AGENTS.md:
# manifest shape, command routability, help/version behavior, stderr
# discipline, and (when the hidden `contract` hook exists) the 0-4 exit-code
# contract. The JSON Schemas in schemas/ are the normative shape definitions;
# this script checks the load-bearing subset with jq so it runs anywhere.
#
# Requires: bash, jq. Exit 0 = conformant, 1 = violations found.

set -u

BIN="${1:?usage: conformance.sh <path-to-binary>}"
FAIL=0

pass() { printf '  \033[32m✓\033[0m %s\n' "$1"; }
fail() { printf '  \033[31m✗\033[0m %s\n' "$1"; FAIL=1; }
note() { printf '  \033[33m·\033[0m %s\n' "$1"; }

# check_json <description> <json> <jq-boolean-filter>
check_json() {
  if printf '%s' "$2" | jq -e "$3" >/dev/null 2>&1; then
    pass "$1"
  else
    fail "$1"
  fi
}

if ! command -v jq >/dev/null 2>&1; then
  echo "conformance.sh requires jq" >&2
  exit 1
fi
if [ ! -x "$BIN" ]; then
  echo "not an executable: $BIN" >&2
  exit 1
fi

echo "== agent-info manifest =="
INFO="$("$BIN" agent-info 2>/dev/null)"
if printf '%s' "$INFO" | jq -e . >/dev/null 2>&1; then
  pass "agent-info emits valid JSON"
else
  fail "agent-info emits valid JSON"
  INFO='{}'
fi
check_json "manifest is raw, not wrapped in the envelope" "$INFO" \
  '(has("data") and has("status")) | not'
check_json "has name, version, description" "$INFO" \
  '(.name|type=="string") and (.version|type=="string") and (.description|type=="string")'
check_json "has non-empty commands object" "$INFO" \
  '(.commands|type=="object") and ((.commands|length) > 0)'
check_json "every command is an object with description/args/options" "$INFO" \
  '[.commands[] | (type=="object") and has("description") and has("args") and has("options")] | all'
check_json "global_flags documents --json and --quiet" "$INFO" \
  '(.global_flags["--json"]|type=="object") and (.global_flags["--quiet"]|type=="object")'
check_json "exit codes 0-4 documented" "$INFO" \
  '[.exit_codes["0","1","2","3","4"]] | all(type=="string")'
check_json "envelope shape documented" "$INFO" \
  '(.envelope.version=="1") and (.envelope|has("success")) and (.envelope|has("error"))'
check_json "config path and env_prefix documented" "$INFO" \
  '(.config.path|type=="string") and (.config.env_prefix|test("^[A-Z][A-Z0-9]*_$"))'
check_json "auto_json_when_piped declared" "$INFO" \
  '.auto_json_when_piped == true'

echo "== help and version =="
"$BIN" --help >/dev/null 2>&1 && pass "--help exits 0" || fail "--help exits 0"
"$BIN" --version >/dev/null 2>&1 && pass "--version exits 0" || fail "--version exits 0"

HELP_OUT="$("$BIN" --help 2>/dev/null)"
check_json "piped --help is wrapped in a success envelope" "$HELP_OUT" \
  '(.status=="success") and (.version=="1")'
check_json "--help teaches usage (Tips + Examples sections)" "$HELP_OUT" \
  '.data.usage | test("Tips:") and test("Examples:")'

echo "== command routability =="
# Every command key in the manifest (possibly multi-word, e.g. "config show")
# must accept --help with exit 0.
while IFS= read -r cmd; do
  # shellcheck disable=SC2086
  if "$BIN" $cmd --help >/dev/null 2>&1; then
    pass "routable: $cmd"
  else
    fail "routable: $cmd"
  fi
done < <(printf '%s' "$INFO" | jq -r '.commands | keys[]' 2>/dev/null)

echo "== error discipline =="
BAD_STDOUT="$("$BIN" definitely-not-a-command-xyz 2>/dev/null)"
BAD_STDERR="$("$BIN" definitely-not-a-command-xyz 2>&1 >/dev/null)"
BAD_CODE=$("$BIN" definitely-not-a-command-xyz >/dev/null 2>&1; echo $?)

[ "$BAD_CODE" = "3" ] && pass "unknown command exits 3" || fail "unknown command exits 3 (got $BAD_CODE)"
[ -z "$BAD_STDOUT" ] && pass "unknown command writes nothing to stdout" || fail "unknown command writes nothing to stdout"
check_json "unknown command emits JSON error envelope on stderr" "$BAD_STDERR" \
  '(.status=="error") and (.error.code|type=="string") and (.error.message|type=="string") and (.error.suggestion|type=="string")'

echo "== exit-code contract =="
# The hidden `contract <code>` hook deterministically triggers each exit code.
# Recommended in every framework CLI; probed only when present.
if "$BIN" contract 0 >/dev/null 2>&1; then
  for code in 1 2 3 4; do
    GOT=$("$BIN" contract "$code" >/dev/null 2>&1; echo $?)
    [ "$GOT" = "$code" ] && pass "contract $code exits $code" || fail "contract $code exits $code (got $GOT)"
  done
else
  note "no hidden 'contract' hook -- exit codes 1-4 not probed (add the hook; see example/)"
fi

echo
if [ "$FAIL" = "0" ]; then
  echo "CONFORMANT: $BIN"
  exit 0
else
  echo "VIOLATIONS FOUND: $BIN"
  exit 1
fi
