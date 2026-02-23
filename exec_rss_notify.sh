#!/usr/bin/env bash
set -euo pipefail

echo "Launching rss notify on $(date +%Y-%m-%d-%H:%M%S)"

echo "Sourcing env vars."
SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
source "${SCRIPT_DIR}"/.env

if [[ ! -f "${RSS_NOTIFY_BIN}" ]]; then
	echo "rss notify is not compiled. Try running \"cargo build --release\" and ensure the bin dir env variable is set to the proper location for your binary."
	exit 1
else
	echo "Killing previous instance of rss notify if it is currently running."
	pkill -f "${RSS_NOTIFY_BIN}" || true
fi

RSS_NOTIFY_LOG_DIR="$(dirname "${RSS_NOTIFY_LOG_FILE}")"

if [[ ! -d "${RSS_NOTIFY_LOG_DIR}" ]]; then
	echo "Logging directory for rss_notify ${RSS_NOTIFY_LOG_DIR} does not exist, making it now."
	mkdir -p "${RSS_NOTIFY_LOG_DIR}"
else
	echo "Performing log cleanup."
	find "${RSS_NOTIFY_LOG_DIR}" -type f -mtime +"${RSS_NOTIFY_LOG_RETENTION_DAYS}" -delete
fi

echo "Making log file ${RSS_NOTIFY_LOG_FILE} for current run."
touch "${RSS_NOTIFY_LOG_FILE}"

echo "Running rss_notify."
"${RSS_NOTIFY_BIN}" >>"${RSS_NOTIFY_LOG_FILE}" 2>&1 &
