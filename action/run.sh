#!/usr/bin/env bash
set -euo pipefail

die() {
  echo "Error: $*" >&2
  exit 1
}

to_lower() {
  printf '%s' "$1" | tr '[:upper:]' '[:lower:]'
}

is_truthy() {
  case "$(to_lower "${1:-}")" in
    1 | true | yes | y | on) return 0 ;;
    *) return 1 ;;
  esac
}

bool_value() {
  if is_truthy "$1"; then
    printf '%s\n' "true"
  else
    printf '%s\n' "false"
  fi
}

is_set() {
  [ -n "${1:-}" ]
}

require_input() {
  local name="$1"
  local value="$2"
  is_set "$value" || die "$name is required for command: $command"
}

append_if_set() {
  local flag="$1"
  local value="$2"
  if is_set "$value"; then
    args+=("$flag" "$value")
  fi
}

append_auth_args() {
  append_if_set "--base-url" "${INPUT_BASE_URL:-}"
  append_if_set "--api-key" "${INPUT_API_KEY:-}"
  append_if_set "--host" "${INPUT_HOST:-}"
  append_if_set "--port" "${INPUT_PORT:-}"
  if is_truthy "${INPUT_INSECURE:-false}"; then
    args+=("--insecure")
  fi
}

emit_multiline_output() {
  local name="$1"
  local file="$2"
  local marker="__1PANEL_CLI_${name}_EOF__"

  if [ -z "${GITHUB_OUTPUT:-}" ]; then
    return
  fi

  {
    echo "${name}<<${marker}"
    cat "$file"
    echo "$marker"
  } >> "$GITHUB_OUTPUT"
}

command="${INPUT_COMMAND:-deploy}"
args=()

if is_truthy "${INPUT_JSON:-true}" && [ "$command" != "help" ]; then
  args+=("--json")
fi

case "$command" in
  help)
    if is_set "${INPUT_HELP_COMMAND:-}"; then
      args+=("${INPUT_HELP_COMMAND}" "--help")
    else
      args+=("--help")
    fi
    ;;

  deploy)
    require_input "path" "${INPUT_PATH:-}"
    require_input "domain" "${INPUT_DOMAIN:-}"
    args+=("deploy")
    append_auth_args
    args+=("--path" "$INPUT_PATH" "--domain" "$INPUT_DOMAIN")
    append_if_set "--group-id" "${INPUT_GROUP_ID:-}"
    append_if_set "--alias" "${INPUT_ALIAS:-}"
    if is_truthy "${INPUT_CREATE_IF_MISSING:-false}"; then
      args+=("--create-if-missing")
    fi
    if is_truthy "${INPUT_YES:-false}"; then
      args+=("--yes")
    fi
    if is_truthy "${INPUT_NON_INTERACTIVE:-true}"; then
      args+=("--non-interactive")
    fi
    ;;

  list-websites)
    args+=("list-websites")
    append_auth_args
    ;;

  list-composes)
    args+=("list-composes")
    append_auth_args
    append_if_set "--info" "${INPUT_INFO:-}"
    ;;

  server-test)
    args+=("server-test")
    append_auth_args
    ;;

  image-export)
    require_input "image-tag" "${INPUT_IMAGE_TAG:-}"
    require_input "output" "${INPUT_OUTPUT:-}"
    args+=("image-export" "--image-tag" "$INPUT_IMAGE_TAG" "--output" "$INPUT_OUTPUT")
    ;;

  image-upload)
    require_input "input" "${INPUT_INPUT:-}"
    args+=("image-upload")
    append_auth_args
    args+=("--input" "$INPUT_INPUT")
    append_if_set "--remote-dir" "${INPUT_REMOTE_DIR:-}"
    ;;

  deploy-load)
    require_input "remote-path" "${INPUT_REMOTE_PATH:-}"
    args+=("deploy-load")
    append_auth_args
    args+=("--remote-path" "$INPUT_REMOTE_PATH")
    ;;

  deploy-compose-update)
    require_input "compose-path" "${INPUT_COMPOSE_PATH:-}"
    require_input "to-image" "${INPUT_TO_IMAGE:-}"
    args+=("deploy-compose-update")
    append_auth_args
    args+=("--compose-path" "$INPUT_COMPOSE_PATH" "--to-image" "$INPUT_TO_IMAGE")
    append_if_set "--compose-name" "${INPUT_COMPOSE_NAME:-}"
    append_if_set "--service" "${INPUT_SERVICE:-}"
    append_if_set "--from-image" "${INPUT_FROM_IMAGE:-}"
    if is_truthy "${INPUT_DRY_RUN:-false}"; then
      args+=("--dry-run")
    fi
    if is_truthy "${INPUT_APPLY:-false}"; then
      args+=("--apply")
    fi
    ;;

  deploy-all)
    require_input "image-tag" "${INPUT_IMAGE_TAG:-}"
    args+=("deploy-all")
    append_auth_args
    args+=("--image-tag" "$INPUT_IMAGE_TAG")
    append_if_set "--remote-dir" "${INPUT_REMOTE_DIR:-}"
    if is_truthy "${INPUT_KEEP_LOCAL_TAR:-false}"; then
      args+=("--keep-local-tar")
    fi
    ;;

  deploy-all-compose)
    require_input "image-tag" "${INPUT_IMAGE_TAG:-}"
    require_input "compose-path" "${INPUT_COMPOSE_PATH:-}"
    args+=("deploy-all-compose")
    append_auth_args
    args+=("--image-tag" "$INPUT_IMAGE_TAG" "--compose-path" "$INPUT_COMPOSE_PATH")
    append_if_set "--remote-dir" "${INPUT_REMOTE_DIR:-}"
    append_if_set "--compose-name" "${INPUT_COMPOSE_NAME:-}"
    append_if_set "--to-image" "${INPUT_TO_IMAGE:-}"
    append_if_set "--service" "${INPUT_SERVICE:-}"
    append_if_set "--from-image" "${INPUT_FROM_IMAGE:-}"
    if is_truthy "${INPUT_KEEP_LOCAL_TAR:-false}"; then
      args+=("--keep-local-tar")
    fi
    if is_set "${INPUT_APPLY:-}"; then
      args+=("--apply" "$(bool_value "$INPUT_APPLY")")
    fi
    ;;

  *)
    die "unsupported command: $command"
    ;;
esac

result_file="${RUNNER_TEMP:-/tmp}/1panel-cli-result-${RANDOM}.txt"

set +e
1panel-cli "${args[@]}" > "$result_file"
status=$?
set -e

cat "$result_file"

if [ "$status" -eq 0 ] && is_truthy "${INPUT_JSON:-true}" && [ "$command" != "help" ]; then
  emit_multiline_output "result-json" "$result_file"
fi

exit "$status"
