#!/usr/bin/env bash
set -euo pipefail

echo "Launching rss_notify on $(date +%Y-%m-%d-%H:%M%S)"

echo "Sourcing env vars."
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
source "${SCRIPT_DIR}"/.env

pkill -f "${RSS_NOTIFY_BIN}" || true

RSS_NOTIFY_LOG_DIR="$(dirname "${RSS_NOTIFY_LOG_FILE}")"

if [[ ! -d "${RSS_NOTIFY_LOG_DIR}" ]]; then
	echo "Logging directory for rss_notify ${RSS_NOTIFY_LOG_DIR} does not exist, making it now."
	mkdir -p "${RSS_NOTIFY_LOG_DIR}"
else
	find "${RSS_NOTIFY_LOG_DIR}" -type f -mtime +"${RSS_NOTIFY_LOG_RETENTION_DAYS}" -delete
fi

echo "Making log file ${RSS_NOTIFY_LOG_FILE} for current run."
touch "${RSS_NOTIFY_LOG_FILE}"

echo "Running rss_notify."
"${RSS_NOTIFY_BIN}" >>"${RSS_NOTIFY_LOG_FILE}" 2>&1 &
