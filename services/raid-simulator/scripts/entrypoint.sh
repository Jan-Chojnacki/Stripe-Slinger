#!/bin/sh
set -eu

AUTH_TOKEN="${GRPC_AUTH_TOKEN:-}"

RAID_LEVEL="${RAID_LEVEL:-raid3}"
DISK_SIZE="${DISK_SIZE:-100000000}"
DISK_DIR="${DISK_DIR:-/disks}"
MOUNT_POINT="${MOUNT_POINT:-/mnt/raid}"

rpcbind

raid-cli fuse \
    --mount-point "$MOUNT_POINT" \
    --disk-dir "$DISK_DIR" \
    --raid "$RAID_LEVEL" \
    --disk-size "$DISK_SIZE" \
    --auth-token "$AUTH_TOKEN" &
FUSE_PID="$!"



echo "Waiting for FUSE mount..."
for _ in $(seq 1 100); do
  mountpoint -q "$MOUNT_POINT" && break
  sleep 0.2
done

if ! mountpoint -q "$MOUNT_POINT"; then
  echo "FUSE mount failed"
  kill "$FUSE_PID" 2>/dev/null || true
  exit 1
fi

echo "$MOUNT_POINT *(rw,fsid=0,async,no_subtree_check,no_root_squash,insecure)" > /etc/exports

exportfs -ra
rpc.nfsd
rpc.mountd -F
