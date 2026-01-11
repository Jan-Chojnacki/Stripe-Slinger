#!/bin/sh
set -eu

SMB_USER="${SMB_USER:-admin}"
SMB_PASS="${SMB_PASS:-changeme}"

METRICS_AUTH_TOKEN="${METRICS_AUTH_TOKEN:-}"

RAID_LEVEL="${RAID_LEVEL:-raid3}"
DISK_SIZE="${DISK_SIZE:-10000000}"
DISK_DIR="${DISK_DIR:-/disks}"
MOUNT_POINT="${MOUNT_POINT:-/mnt/raid}"

if ! id "$SMB_USER" >/dev/null 2>&1; then
  useradd -M -s /usr/sbin/nologin "$SMB_USER"
fi

set +x
(echo "$SMB_PASS"; echo "$SMB_PASS") | smbpasswd -a -s "$SMB_USER"
smbpasswd -e "$SMB_USER" >/dev/null 2>&1 || true
set -x

runuser -u nonroot --preserve-environment -- \
  raid-cli fuse \
    --mount-point "$MOUNT_POINT" \
    --disk-dir "$DISK_DIR" \
    --raid "$RAID_LEVEL" \
    --disk-size "$DISK_SIZE" \
    --auth-token "$METRICS_AUTH_TOKEN" &
FUSE_PID="$!"

for _ in $(seq 1 100); do
  mountpoint -q "$MOUNT_POINT" && break
  sleep 0.1
done

mountpoint -q "$MOUNT_POINT" || {
  kill "$FUSE_PID" 2>/dev/null || true
  exit 1
}

trap 'kill "$FUSE_PID" 2>/dev/null || true' TERM INT

exec smbd -F --no-process-group
