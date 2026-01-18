#!/bin/bash
set -euo pipefail

cleanup() {
  echo "Stopping services..."
  exportfs -uav
  service nfs-kernel-server stop
  service rpcbind stop
  if [ -n "$FUSE_PID" ]; then
    kill "$FUSE_PID" 2>/dev/null || true
  fi
  umount -l "$MOUNT_POINT" 2>/dev/null || true
  exit 0
}

trap cleanup SIGTERM SIGINT

AUTH_TOKEN="${GRPC_AUTH_TOKEN:-}"
RAID_LEVEL="${RAID_LEVEL:-raid3}"
DISK_SIZE="${DISK_SIZE:-100000000}"
DISK_DIR="${DISK_DIR:-/disks}"
MOUNT_POINT="${MOUNT_POINT:-/mnt/raid}"

rpcbind

echo "user_allow_other" >> /etc/fuse.conf

raid-cli fuse \
    --mount-point "$MOUNT_POINT" \
    --disk-dir "$DISK_DIR" \
    --raid "$RAID_LEVEL" \
    --disk-size "$DISK_SIZE" \
    --auth-token "$AUTH_TOKEN" \
    --allow-other &

echo "Waiting for FUSE mount at $MOUNT_POINT..."
for _ in {1..50}; do
  if mountpoint -q "$MOUNT_POINT"; then
    echo "FUSE mounted successfully."
    break
  fi
  sleep 0.2
  if ! kill -0 "$FUSE_PID" 2>/dev/null; then
    echo "FUSE process died unexpectedly."
    exit 1
  fi
done

if ! mountpoint -q "$MOUNT_POINT"; then
  echo "Timeout waiting for FUSE mount."
  exit 1
fi

echo "$MOUNT_POINT *(rw,fsid=0,async,no_subtree_check,no_root_squash,insecure)" > /etc/exports

exportfs -ra
rpc.nfsd --debug 8 --no-udp
echo "NFS started."

rpc.mountd -F &
MOUNTD_PID=$!

wait "$MOUNTD_PID"
