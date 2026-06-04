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

download() {
  local url="$1"
  local output="$2"
  local -a curl_args=(-fsSL --retry 3 --retry-delay 2)

  if [ -n "${INPUT_GITHUB_TOKEN:-}" ]; then
    curl_args+=(-H "Authorization: Bearer ${INPUT_GITHUB_TOKEN}")
  fi

  curl "${curl_args[@]}" "$url" -o "$output"
}

detect_target() {
  if [ "$(uname -s)" != "Linux" ]; then
    die "automatic release download currently supports Linux runners only. Pass binary-path or binary-url for this runner."
  fi

  case "$(uname -m)" in
    x86_64 | amd64) printf '%s\n' "x86_64-unknown-linux-musl" ;;
    aarch64 | arm64) printf '%s\n' "aarch64-unknown-linux-musl" ;;
    *) die "unsupported runner architecture: $(uname -m)" ;;
  esac
}

resolve_version() {
  if [ -n "${INPUT_VERSION:-}" ]; then
    printf '%s\n' "$INPUT_VERSION"
    return
  fi

  local ref="${INPUT_ACTION_REF:-${GITHUB_ACTION_REF:-}}"
  if [ -n "$ref" ] && [[ "$ref" == v* ]]; then
    printf '%s\n' "$ref"
    return
  fi

  printf '%s\n' "latest"
}

verify_checksum() {
  local checksum_url="$1"
  local asset_name="$2"
  local binary="$3"
  local checksum_file="$4"

  if ! is_truthy "${INPUT_VERIFY_CHECKSUM:-true}"; then
    return
  fi

  if ! download "$checksum_url" "$checksum_file"; then
    echo "Checksum file is not available; skipping checksum verification." >&2
    return
  fi

  local expected
  expected="$(awk -v name="$asset_name" '$2 ~ name "$" { print $1; exit }' "$checksum_file")"
  if [ -z "$expected" ]; then
    echo "Checksum for $asset_name was not found; skipping checksum verification." >&2
    return
  fi

  local actual
  actual="$(sha256sum "$binary" | awk '{ print $1 }')"
  if [ "$actual" != "$expected" ]; then
    die "checksum mismatch for $asset_name"
  fi
}

install_dir="${RUNNER_TEMP:-/tmp}/1panel-cli-action"
mkdir -p "$install_dir"
binary="$install_dir/1panel-cli"

if [ -n "${INPUT_BINARY_PATH:-}" ]; then
  [ -f "$INPUT_BINARY_PATH" ] || die "binary-path does not exist: $INPUT_BINARY_PATH"
  cp "$INPUT_BINARY_PATH" "$binary"
elif [ -n "${INPUT_BINARY_URL:-}" ]; then
  download "$INPUT_BINARY_URL" "$binary"
else
  repo="${INPUT_REPOSITORY:-${INPUT_ACTION_REPOSITORY:-${GITHUB_ACTION_REPOSITORY:-}}}"
  [ -n "$repo" ] || die "repository is required when binary-url and binary-path are empty"

  target="${INPUT_TARGET:-$(detect_target)}"
  asset_name="1panel-cli-${target}"
  version="$(resolve_version)"

  if [ "$version" = "latest" ]; then
    binary_url="https://github.com/${repo}/releases/latest/download/${asset_name}"
    checksum_url="https://github.com/${repo}/releases/latest/download/SHA256SUMS"
  else
    binary_url="https://github.com/${repo}/releases/download/${version}/${asset_name}"
    checksum_url="https://github.com/${repo}/releases/download/${version}/SHA256SUMS"
  fi

  echo "Downloading $asset_name from $repo ($version)"
  download "$binary_url" "$binary"
  verify_checksum "$checksum_url" "$asset_name" "$binary" "$install_dir/SHA256SUMS"
fi

chmod +x "$binary"

if [ -n "${GITHUB_PATH:-}" ]; then
  echo "$install_dir" >> "$GITHUB_PATH"
fi

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "binary-path=$binary" >> "$GITHUB_OUTPUT"
fi

"$binary" --version
