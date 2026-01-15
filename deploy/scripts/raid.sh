#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

COMPOSE_FILE="$REPO_ROOT/deploy/docker-compose.yml"
MOUNT_POINT="$REPO_ROOT/storage/raid-data-host"

NFS_PORT=2049

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

wait_for_port() {
  local port=$1
  local max_retries=30
  local count=0

  log "Waiting for NFS port ($port) to be reachable..."

  while ! (echo > /dev/tcp/localhost/$port) >/dev/null 2>&1; do
    sleep 1
    count=$((count + 1))
    if [ $count -ge $max_retries ]; then
      return 1
    fi
  done
  return 0
}

case "$1" in
  up)
    log "Starting Docker containers using config: $COMPOSE_FILE"

    mkdir -p "$MOUNT_POINT"
    mkdir -p "$REPO_ROOT/storage/raid-disks"
    mkdir -p "$REPO_ROOT/storage/alloy-data"

    docker compose -f "$COMPOSE_FILE" up -d

    if ! wait_for_port $NFS_PORT; then
        error "Timeout: NFS server did not start in time."
        exit 1
    fi

    sleep 2

    log "Mounting RAID on host..."
    if mountpoint -q "$MOUNT_POINT"; then
      warn "Target directory is already mounted."
    else
      if sudo mount -t nfs -o port=$NFS_PORT,nolock,tcp,resvport,actimeo=0,noac,lookupcache=none localhost:/ "$MOUNT_POINT"; then
        log "Success! RAID is now available at: $(realpath $MOUNT_POINT)"

        log "Warming up RAID controller..."
        timeout 1s bash -c "echo 'init' > $MOUNT_POINT/.raidctl" 2>/dev/null || true

      else
        error "Failed to mount NFS share."
        exit 1
      fi
    fi
    ;;

  down)
    log "Stopping the environment..."

    if mountpoint -q "$MOUNT_POINT"; then
      log "Unmounting the RAID directory from host..."
      sudo umount -l "$MOUNT_POINT"
    else
      warn "RAID directory was not mounted."
    fi

    log "Stopping Docker containers..."
    docker compose -f "$COMPOSE_FILE" down
    log "Done."
    ;;

  status)
    if mountpoint -q "$MOUNT_POINT"; then
      log "RAID MOUNT: [OK] -> $MOUNT_POINT"
      log "Listing files:"
      ls -F "$MOUNT_POINT"
    else
      warn "RAID MOUNT: [NOT MOUNTED]"
    fi
    echo "-----------------------------------"
    docker compose -f "$COMPOSE_FILE" ps
    ;;

  *)
    echo "Usage: $0 {up|down|status}"
    exit 1
    ;;
esac
